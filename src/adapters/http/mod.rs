pub mod dtos;
pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod routes;

// Re-export commonly used types
pub use dtos::{
  CurrentUserResponse, ErrorResponse, LoginRequest, LoginResponse, LogoutAllResponse,
  RegisterRequest, RegisterResponse, SuccessResponse,
};
pub use errors::{ApiError, AuthErrorKind};
pub use handlers::auth::{
  get_current_user_handler, login_handler, logout_all_handler, logout_handler, register_handler,
};
pub use middleware::{AuthMiddleware, AuthUser, RequestId, RequestIdExt, RequestIdMiddleware};
pub use routes::configure_auth_routes;
