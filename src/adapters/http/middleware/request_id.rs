use actix_web::{
  Error, HttpMessage,
  body::MessageBody,
  dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;
use std::{
  future::{Ready, ready},
  rc::Rc,
};
use uuid::Uuid;

/// Request ID middleware that generates a unique ID for each request
///
/// This middleware:
/// 1. Generates a UUID v4 for each incoming request
/// 2. Adds the ID to response headers as X-Request-ID
/// 3. Stores the ID in request extensions for use in tracing/logging
///
/// The request ID can be used to:
/// - Track requests across services
/// - Correlate logs for a single request
/// - Debug specific requests in production
///
/// # Example
///
/// ```no_run
/// use actix_web::App;
/// # use taxbyte::adapters::http::middleware::request_id::RequestIdMiddleware;
///
/// let app = App::new()
///   .wrap(RequestIdMiddleware::default());
/// ```
///
/// # Accessing Request ID in Handlers
///
/// ```ignore
/// use actix_web::{web, HttpRequest, HttpResponse};
/// use uuid::Uuid;
///
/// async fn handler(req: HttpRequest) -> HttpResponse {
///   let request_id = req.extensions().get::<Uuid>().copied();
///   HttpResponse::Ok().body(format!("Request ID: {:?}", request_id))
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct RequestIdMiddleware;

impl RequestIdMiddleware {
  /// Creates a new request ID middleware
  pub fn new() -> Self {
    Self
  }
}

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: MessageBody + 'static,
{
  type Response = ServiceResponse<B>;
  type Error = Error;
  type Transform = RequestIdMiddlewareService<S>;
  type InitError = ();
  type Future = Ready<Result<Self::Transform, Self::InitError>>;

  fn new_transform(&self, service: S) -> Self::Future {
    ready(Ok(RequestIdMiddlewareService {
      service: Rc::new(service),
    }))
  }
}

pub struct RequestIdMiddlewareService<S> {
  service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddlewareService<S>
where
  S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
  S::Future: 'static,
  B: MessageBody + 'static,
{
  type Response = ServiceResponse<B>;
  type Error = Error;
  type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

  forward_ready!(service);

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let service = Rc::clone(&self.service);

    Box::pin(async move {
      // Generate a new request ID
      let request_id = RequestId(Uuid::new_v4());

      // Store request ID in extensions for logging/tracing
      req.extensions_mut().insert(request_id);

      // Add request ID to tracing span
      tracing::Span::current().record("request_id", request_id.0.to_string());

      // Call the next service
      let mut res = service.call(req).await?;

      // Add request ID to response headers
      res.headers_mut().insert(
        actix_web::http::header::HeaderName::from_static("x-request-id"),
        actix_web::http::header::HeaderValue::from_str(&request_id.0.to_string())
          .unwrap_or_else(|_| actix_web::http::header::HeaderValue::from_static("invalid-uuid")),
      );

      Ok(res)
    })
  }
}

/// Request ID wrapper for UUID
///
/// This type is stored in request extensions and can be retrieved by handlers.
#[derive(Debug, Clone, Copy)]
pub struct RequestId(pub Uuid);

impl RequestId {
  /// Creates a new request ID
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Returns the UUID value
  pub fn value(&self) -> Uuid {
    self.0
  }

  /// Returns the request ID as a string
  pub fn as_str(&self) -> String {
    self.0.to_string()
  }
}

impl Default for RequestId {
  fn default() -> Self {
    Self::new()
  }
}

impl std::fmt::Display for RequestId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

/// Extension trait to easily extract request ID from request
pub trait RequestIdExt {
  /// Get the request ID from request extensions
  ///
  /// Returns None if the request ID is not present (middleware not configured).
  fn request_id(&self) -> Option<RequestId>;
}

impl RequestIdExt for actix_web::HttpRequest {
  fn request_id(&self) -> Option<RequestId> {
    self.extensions().get::<RequestId>().cloned()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use actix_web::{
    App, HttpResponse,
    test::{self, TestRequest},
    web,
  };

  #[actix_web::test]
  async fn test_request_id_middleware() {
    async fn test_handler(req: actix_web::HttpRequest) -> HttpResponse {
      let request_id = req.extensions().get::<RequestId>().cloned();
      assert!(request_id.is_some());
      HttpResponse::Ok().finish()
    }

    let app = test::init_service(
      App::new()
        .wrap(RequestIdMiddleware::new())
        .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&app, req).await;

    // Check that response has X-Request-ID header
    assert!(resp.headers().contains_key("x-request-id"));

    // Verify the header value is a valid UUID
    let request_id = resp.headers().get("x-request-id").unwrap();
    let request_id_str = request_id.to_str().unwrap();
    assert!(Uuid::parse_str(request_id_str).is_ok());
  }

  #[test]
  fn test_request_id_creation() {
    let id1 = RequestId::new();
    let id2 = RequestId::new();

    // Each request ID should be unique
    assert_ne!(id1.value(), id2.value());
  }

  #[test]
  fn test_request_id_display() {
    let id = RequestId::new();
    let display = format!("{}", id);
    let as_str = id.as_str();

    assert_eq!(display, as_str);
    assert!(Uuid::parse_str(&display).is_ok());
  }
}
