use std::sync::Arc;
use uuid::Uuid;

use crate::domain::auth::errors::AuthError;
use crate::domain::auth::services::AuthService;

/// Response after logging out from all devices
#[derive(Debug, Clone)]
pub struct LogoutAllDevicesResponse {
  /// Number of sessions that were terminated
  pub sessions_terminated: usize,
}

/// Use case for logging out a user from all devices
pub struct LogoutAllDevicesUseCase {
  auth_service: Arc<AuthService>,
}

impl LogoutAllDevicesUseCase {
  /// Creates a new instance of LogoutAllDevicesUseCase
  pub fn new(auth_service: Arc<AuthService>) -> Self {
    Self { auth_service }
  }

  /// Executes the logout all devices use case
  ///
  /// # Arguments
  /// * `user_id` - The ID of the user whose sessions should be terminated
  ///
  /// # Returns
  /// A `LogoutAllDevicesResponse` containing the number of sessions terminated
  ///
  /// # Errors
  /// Returns `AuthError` if the operation fails (e.g., user not found)
  pub async fn execute(&self, user_id: Uuid) -> Result<LogoutAllDevicesResponse, AuthError> {
    // Logout from all sessions using the auth service
    let sessions_terminated = self.auth_service.logout_all(user_id).await?;

    // Build and return the response
    Ok(LogoutAllDevicesResponse {
      sessions_terminated,
    })
  }
}
