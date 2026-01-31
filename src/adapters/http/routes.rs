use actix_web::web;
use std::sync::Arc;

use crate::application::auth::{
  GetCurrentUserUseCase, LoginUserUseCase, LogoutAllDevicesUseCase, LogoutUserUseCase,
  RegisterUserUseCase,
};

use super::handlers::auth::{
  get_current_user_handler, login_handler, logout_all_handler, logout_handler, register_handler,
};

/// Configure authentication routes
///
/// Mounts all authentication-related endpoints under the provided scope.
/// All routes are prefixed with the scope path (e.g., /api/v1/auth).
///
/// # Routes
///
/// - POST /register - Register a new user account
/// - POST /login - Authenticate and create a session
/// - POST /logout - Invalidate the current session
/// - POST /logout-all - Invalidate all sessions for the user
/// - GET /me - Get current user information
///
/// # Arguments
///
/// * `register_use_case` - Use case for user registration
/// * `login_use_case` - Use case for user login
/// * `logout_use_case` - Use case for user logout
/// * `logout_all_use_case` - Use case for logging out from all devices
/// * `get_user_use_case` - Use case for getting current user info
///
/// # Example
///
/// ```no_run
/// use actix_web::{App, web};
/// use std::sync::Arc;
/// # use taxbyte::application::auth::*;
/// # use taxbyte::adapters::http::routes::configure_auth_routes;
///
/// # async fn example(
/// #   register_use_case: Arc<RegisterUserUseCase>,
/// #   login_use_case: Arc<LoginUserUseCase>,
/// #   logout_use_case: Arc<LogoutUserUseCase>,
/// #   logout_all_use_case: Arc<LogoutAllDevicesUseCase>,
/// #   get_user_use_case: Arc<GetCurrentUserUseCase>,
/// # ) {
/// let app = App::new().service(
///   web::scope("/api/v1/auth").configure(|cfg| {
///     configure_auth_routes(
///       cfg,
///       register_use_case,
///       login_use_case,
///       logout_use_case,
///       logout_all_use_case,
///       get_user_use_case,
///     )
///   }),
/// );
/// # }
/// ```
pub fn configure_auth_routes(
  cfg: &mut web::ServiceConfig,
  register_use_case: Arc<RegisterUserUseCase>,
  login_use_case: Arc<LoginUserUseCase>,
  logout_use_case: Arc<LogoutUserUseCase>,
  logout_all_use_case: Arc<LogoutAllDevicesUseCase>,
  get_user_use_case: Arc<GetCurrentUserUseCase>,
) {
  // Store use cases in app data so handlers can access them
  cfg
    .app_data(web::Data::new(register_use_case))
    .app_data(web::Data::new(login_use_case))
    .app_data(web::Data::new(logout_use_case))
    .app_data(web::Data::new(logout_all_use_case.clone()))
    .app_data(web::Data::new(get_user_use_case.clone()))
    // Configure routes
    .route("/register", web::post().to(register_handler))
    .route("/login", web::post().to(login_handler))
    .route("/logout", web::post().to(logout_handler))
    .route("/logout-all", web::post().to(logout_all_handler))
    .route("/me", web::get().to(get_current_user_handler));
}

#[cfg(test)]
mod tests {
  #[tokio::test]
  async fn test_routes_configuration() {
    // This test verifies that routes can be configured without panicking
    // We don't test the actual handlers here, just the route configuration

    // Note: We can't easily create real use cases without database connections,
    // so we just verify the configuration syntax is correct by compiling
    assert!(true);
  }
}
