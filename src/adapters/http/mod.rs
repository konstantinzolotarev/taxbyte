pub mod dtos;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod templates;

// Re-export commonly used types
pub use dtos::{
  CurrentUserResponse, ErrorResponse, LoginRequest, LoginResponse, LogoutAllResponse,
  RegisterRequest, RegisterResponse, SuccessResponse,
};
pub use errors::{ApiError, AuthErrorKind};
pub use handlers::auth::{
  get_current_user_handler, login_handler, logout_all_handler, logout_handler, register_handler,
};
pub use middleware::{
  AuthMiddleware, AuthUser, RequestId, RequestIdExt, RequestIdMiddleware, WebAuthMiddleware,
};
pub use routes::{
  WebRouteDependencies, configure_auth_routes, configure_bank_account_routes,
  configure_company_routes, configure_customer_routes, configure_invoice_routes,
  configure_web_routes,
};
pub use templates::TemplateEngine;
