use actix_web::{
  Error, HttpMessage, HttpResponse,
  body::EitherBody,
  dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;
use std::{
  future::{Ready, ready},
  rc::Rc,
  sync::Arc,
};
use uuid::Uuid;

use crate::{
  adapters::http::errors::ApiError,
  domain::{
    auth::entities::User,
    company::{
      entities::CompanyRole,
      ports::{ActiveCompanyRepository, CompanyMemberRepository},
    },
  },
};

/// Company context extracted from URL and validated against user membership
#[derive(Debug, Clone)]
pub struct CompanyContext {
  pub company_id: Uuid,
  pub role: CompanyRole,
}

/// Middleware that extracts company_id from URL path and validates user membership
///
/// This middleware:
/// 1. Expects a User to be present in request extensions (set by WebAuthMiddleware)
/// 2. Extracts company_id from URL path parameter
/// 3. Validates that the user is a member of the company
/// 4. Attaches CompanyContext to request extensions for downstream handlers
/// 5. Optionally updates active_companies table (fire-and-forget) for UX
///
/// # Example
///
/// ```no_run
/// use actix_web::{App, web};
/// use std::sync::Arc;
/// # use taxbyte::adapters::http::middleware::company_context::CompanyContextMiddleware;
/// # use taxbyte::domain::company::ports::{CompanyMemberRepository, ActiveCompanyRepository};
///
/// # async fn example(
/// #   member_repo: Arc<dyn CompanyMemberRepository>,
/// #   active_repo: Arc<dyn ActiveCompanyRepository>
/// # ) {
/// let app = App::new()
///   .service(
///     web::scope("/c/{company_id}")
///       .wrap(CompanyContextMiddleware::new(member_repo, active_repo))
///       .route("/invoices", web::get().to(|| async { "Invoices" }))
///   );
/// # }
/// ```
pub struct CompanyContextMiddleware {
  member_repo: Arc<dyn CompanyMemberRepository>,
  active_repo: Arc<dyn ActiveCompanyRepository>,
}

impl CompanyContextMiddleware {
  /// Creates a new company context middleware
  ///
  /// # Arguments
  ///
  /// * `member_repo` - Repository for checking company membership
  /// * `active_repo` - Repository for updating last-used company
  pub fn new(
    member_repo: Arc<dyn CompanyMemberRepository>,
    active_repo: Arc<dyn ActiveCompanyRepository>,
  ) -> Self {
    Self {
      member_repo,
      active_repo,
    }
  }
}

impl<S, B> Transform<S, ServiceRequest> for CompanyContextMiddleware
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<EitherBody<B>>;
  type Error = Error;
  type Transform = CompanyContextMiddlewareService<S>;
  type InitError = ();
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(CompanyContextMiddlewareService {
      service: Rc::new(service),
      member_repo: self.member_repo.clone(),
      active_repo: self.active_repo.clone(),
    }))
  }
}

pub struct CompanyContextMiddlewareService<S> {
  service: Rc<S>,
  member_repo: Arc<dyn CompanyMemberRepository>,
  active_repo: Arc<dyn ActiveCompanyRepository>,
}

impl<S, B> Service<ServiceRequest> for CompanyContextMiddlewareService<S>
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<EitherBody<B>>;
  type Error = Error;
  type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

  forward_ready!(service);

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let service = Rc::clone(&self.service);
    let member_repo = self.member_repo.clone();
    let active_repo = self.active_repo.clone();

    Box::pin(async move {
      // 1. Extract user from extensions (set by WebAuthMiddleware)
      let user_opt = req.extensions().get::<User>().cloned();

      let user = match user_opt {
        Some(user) => user,
        None => {
          // WebAuthMiddleware should have handled this, but if we get here,
          // redirect to login for web requests
          let res = req.into_response(
            HttpResponse::Found()
              .insert_header(("Location", "/login"))
              .finish(),
          );
          return Ok(res.map_into_right_body());
        }
      };

      // 2. Extract company_id from path parameter
      let company_id_result = extract_company_id(&req);

      let company_id = match company_id_result {
        Ok(id) => id,
        Err(_) => {
          // Invalid company ID format - redirect to companies page
          let res = req.into_response(
            HttpResponse::Found()
              .insert_header(("Location", "/companies"))
              .finish(),
          );
          return Ok(res.map_into_right_body());
        }
      };

      // 3. Validate membership
      let member = match member_repo.find_member(company_id, user.id).await {
        Ok(Some(member)) => member,
        Ok(None) => {
          // User is not a member of this company - redirect to companies page
          let res = req.into_response(
            HttpResponse::Found()
              .insert_header(("Location", "/companies"))
              .finish(),
          );
          return Ok(res.map_into_right_body());
        }
        Err(_) => {
          // Database error - redirect to companies page
          let res = req.into_response(
            HttpResponse::Found()
              .insert_header(("Location", "/companies"))
              .finish(),
          );
          return Ok(res.map_into_right_body());
        }
      };

      // 4. Attach CompanyContext to request extensions
      let context = CompanyContext {
        company_id,
        role: member.role,
      };
      req.extensions_mut().insert(context);

      // 5. Fire-and-forget: Update active_companies for "last used" tracking
      let active_repo_clone = active_repo.clone();
      let user_id = user.id;
      tokio::spawn(async move {
        use crate::domain::company::entities::ActiveCompany;
        let _ = active_repo_clone
          .set_active(ActiveCompany::new(user_id, company_id))
          .await;
      });

      // 6. Call the next service
      let res = service.call(req).await?;
      Ok(res.map_into_left_body())
    })
  }
}

/// Extract company_id from URL path parameter
fn extract_company_id(req: &ServiceRequest) -> Result<Uuid, ApiError> {
  req
    .match_info()
    .get("company_id")
    .ok_or_else(|| ApiError::Validation("Missing company_id in URL path".to_string()))
    .and_then(|id_str| {
      Uuid::parse_str(id_str)
        .map_err(|_| ApiError::Validation(format!("Invalid company_id format: {}", id_str)))
    })
}

/// Extension trait to easily extract company context from request
pub trait CompanyContextExt {
  /// Get the company context from request extensions
  ///
  /// # Panics
  ///
  /// Panics if the context is not present in extensions.
  /// This should only be called in handlers that are protected by CompanyContextMiddleware.
  fn company_context(&self) -> CompanyContext;
}

impl CompanyContextExt for actix_web::HttpRequest {
  fn company_context(&self) -> CompanyContext {
    self
      .extensions()
      .get::<CompanyContext>()
      .cloned()
      .expect(
        "CompanyContext not found in request extensions. Did you forget to add CompanyContextMiddleware?"
      )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use actix_web::test::TestRequest;

  #[test]
  fn test_extract_company_id_valid() {
    let company_id = Uuid::new_v4();
    let req = TestRequest::default()
      .param("company_id", &company_id.to_string())
      .to_srv_request();

    let extracted = extract_company_id(&req).unwrap();
    assert_eq!(extracted, company_id);
  }

  #[test]
  fn test_extract_company_id_missing() {
    let req = TestRequest::default().to_srv_request();
    let result = extract_company_id(&req);
    assert!(result.is_err());
  }

  #[test]
  fn test_extract_company_id_invalid_format() {
    let req = TestRequest::default()
      .param("company_id", "not-a-uuid")
      .to_srv_request();

    let result = extract_company_id(&req);
    assert!(result.is_err());
  }
}
