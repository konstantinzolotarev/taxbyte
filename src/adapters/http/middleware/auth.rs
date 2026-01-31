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

use crate::{
  adapters::http::errors::{ApiError, AuthErrorKind},
  application::auth::GetCurrentUserUseCase,
  domain::auth::entities::User,
};

/// Authentication middleware that validates session tokens and attaches user to request
///
/// This middleware:
/// 1. Extracts the session token from the Authorization header
/// 2. Validates the token using GetCurrentUserUseCase
/// 3. Attaches the User entity to request extensions for downstream handlers
/// 4. Returns 401 Unauthorized if the token is invalid or expired
///
/// # Example
///
/// ```no_run
/// use actix_web::{App, web};
/// use std::sync::Arc;
/// # use taxbyte::application::auth::GetCurrentUserUseCase;
/// # use taxbyte::adapters::http::middleware::auth::AuthMiddleware;
///
/// # async fn example(get_user_use_case: Arc<GetCurrentUserUseCase>) {
/// let app = App::new()
///   .wrap(AuthMiddleware::new(get_user_use_case))
///   .service(
///     web::resource("/protected")
///       .route(web::get().to(|| async { "Protected endpoint" }))
///   );
/// # }
/// ```
pub struct AuthMiddleware {
  get_user_use_case: Arc<GetCurrentUserUseCase>,
}

impl AuthMiddleware {
  /// Creates a new authentication middleware
  ///
  /// # Arguments
  ///
  /// * `get_user_use_case` - Use case for retrieving user information from session token
  pub fn new(get_user_use_case: Arc<GetCurrentUserUseCase>) -> Self {
    Self { get_user_use_case }
  }
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<EitherBody<B>>;
  type Error = Error;
  type Transform = AuthMiddlewareService<S>;
  type InitError = ();
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(AuthMiddlewareService {
      service: Rc::new(service),
      get_user_use_case: self.get_user_use_case.clone(),
    }))
  }
}

pub struct AuthMiddlewareService<S> {
  service: Rc<S>,
  get_user_use_case: Arc<GetCurrentUserUseCase>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
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
    let get_user_use_case = self.get_user_use_case.clone();

    Box::pin(async move {
      // Extract session token from Authorization header
      let session_token = match extract_session_token(&req) {
        Ok(token) => token,
        Err(e) => {
          let (request, _) = req.into_parts();
          let response = HttpResponse::Unauthorized().json(e).map_into_right_body();
          return Ok(ServiceResponse::new(request, response));
        }
      };

      // Validate token and get user
      let user_response = match get_user_use_case.execute(session_token).await {
        Ok(response) => response,
        Err(e) => {
          let (request, _) = req.into_parts();
          let api_error: ApiError = e.into();
          let response = HttpResponse::Unauthorized()
            .json(api_error)
            .map_into_right_body();
          return Ok(ServiceResponse::new(request, response));
        }
      };

      // Create User entity from response
      let user = User::from_db(
        user_response.user_id,
        user_response.email,
        String::new(), // We don't need the password hash in the middleware
        String::new(), // We don't need the full name in the middleware
        false,         // Email verification status not needed
        None,
        None,
        None,
        None,
        user_response.created_at,
        chrono::Utc::now(), // Updated at not needed
      );

      // Attach user to request extensions
      req.extensions_mut().insert(user);

      // Call the next service
      let res = service.call(req).await?;
      Ok(res.map_into_left_body())
    })
  }
}

/// Extract session token from Authorization header
fn extract_session_token(req: &ServiceRequest) -> Result<String, ApiError> {
  req
    .headers()
    .get("Authorization")
    .and_then(|h| h.to_str().ok())
    .and_then(|s| s.strip_prefix("Bearer "))
    .map(|s| s.to_string())
    .ok_or(ApiError::Auth(AuthErrorKind::InvalidToken))
}

/// Extension trait to easily extract authenticated user from request
pub trait AuthUser {
  /// Get the authenticated user from request extensions
  ///
  /// # Panics
  ///
  /// Panics if the user is not present in extensions.
  /// This should only be called in handlers that are protected by AuthMiddleware.
  fn authenticated_user(&self) -> User;
}

impl AuthUser for actix_web::HttpRequest {
  fn authenticated_user(&self) -> User {
    self
      .extensions()
      .get::<User>()
      .cloned()
      .expect("User not found in request extensions. Did you forget to add AuthMiddleware?")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use actix_web::test::TestRequest;

  #[test]
  fn test_extract_session_token_valid() {
    let req = TestRequest::default()
      .insert_header(("Authorization", "Bearer test_token_123"))
      .to_srv_request();

    let token = extract_session_token(&req).unwrap();
    assert_eq!(token, "test_token_123");
  }

  #[test]
  fn test_extract_session_token_missing() {
    let req = TestRequest::default().to_srv_request();

    let result = extract_session_token(&req);
    assert!(result.is_err());
  }

  #[test]
  fn test_extract_session_token_invalid_format() {
    let req = TestRequest::default()
      .insert_header(("Authorization", "InvalidFormat token"))
      .to_srv_request();

    let result = extract_session_token(&req);
    assert!(result.is_err());
  }
}
