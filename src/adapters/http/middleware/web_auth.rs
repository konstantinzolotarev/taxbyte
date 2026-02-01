use actix_web::{
  body::EitherBody,
  dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
  Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::{future::ready, rc::Rc, sync::Arc};

use crate::domain::auth::services::AuthService;
use crate::domain::auth::value_objects::SessionToken;

/// Web authentication middleware using cookie-based sessions
pub struct WebAuthMiddleware {
  auth_service: Arc<AuthService>,
}

impl WebAuthMiddleware {
  pub fn new(auth_service: Arc<AuthService>) -> Self {
    Self { auth_service }
  }
}

impl<S, B> Transform<S, ServiceRequest> for WebAuthMiddleware
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: 'static,
{
  type Response = ServiceResponse<EitherBody<B>>;
  type Error = Error;
  type InitError = ();
  type Transform = WebAuthMiddlewareService<S>;
  type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(WebAuthMiddlewareService {
      service: Rc::new(service),
      auth_service: self.auth_service.clone(),
    }))
  }
}

pub struct WebAuthMiddlewareService<S> {
  service: Rc<S>,
  auth_service: Arc<AuthService>,
}

impl<S, B> Service<ServiceRequest> for WebAuthMiddlewareService<S>
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
    // Extract session token from cookie
    let token = req.cookie("session_token").map(|c| c.value().to_string());

    let auth_service = self.auth_service.clone();
    let service = Rc::clone(&self.service);

    Box::pin(async move {
      if let Some(token_str) = token {
        match SessionToken::from_string(token_str) {
          Ok(session_token) => match auth_service.validate_session(session_token).await {
            Ok(user) => {
              // Attach user to request extensions
              req.extensions_mut().insert(user);
              let res = service.call(req).await?;
              Ok(res.map_into_left_body())
            }
            Err(_) => {
              // Invalid session - redirect to login for web requests
              let res = req.into_response(
                HttpResponse::Found()
                  .insert_header(("Location", "/login"))
                  .finish(),
              );
              Ok(res.map_into_right_body())
            }
          },
          Err(_) => {
            // Invalid token format
            let res = req.into_response(
              HttpResponse::Found()
                .insert_header(("Location", "/login"))
                .finish(),
            );
            Ok(res.map_into_right_body())
          }
        }
      } else {
        // No session cookie - redirect to login
        let res = req.into_response(
          HttpResponse::Found()
            .insert_header(("Location", "/login"))
            .finish(),
        );
        Ok(res.map_into_right_body())
      }
    })
  }
}
