use std::sync::Arc;

use crate::domain::auth::errors::AuthError;
use crate::domain::auth::services::AuthService;
use crate::domain::auth::value_objects::SessionToken;

/// Use case for logging out a user
pub struct LogoutUserUseCase {
  auth_service: Arc<AuthService>,
}

impl LogoutUserUseCase {
  /// Creates a new instance of LogoutUserUseCase
  pub fn new(auth_service: Arc<AuthService>) -> Self {
    Self { auth_service }
  }

  /// Executes the user logout use case
  ///
  /// # Arguments
  /// * `session_token` - The session token to invalidate
  ///
  /// # Returns
  /// Ok(()) if the logout was successful
  ///
  /// # Errors
  /// Returns `AuthError` if logout fails (e.g., invalid session token)
  pub async fn execute(&self, session_token: String) -> Result<(), AuthError> {
    // Parse and validate session token
    let token = SessionToken::from_string(session_token)?;

    // Logout the user using the auth service
    self.auth_service.logout(token).await?;

    Ok(())
  }
}
