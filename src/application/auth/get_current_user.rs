use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::auth::errors::AuthError;
use crate::domain::auth::services::AuthService;
use crate::domain::auth::value_objects::SessionToken;

/// Response containing current user information
#[derive(Debug, Clone)]
pub struct GetCurrentUserResponse {
  /// Unique identifier of the user
  pub user_id: Uuid,
  /// User's email address
  pub email: String,
  /// Timestamp when the user account was created
  pub created_at: DateTime<Utc>,
  /// Timestamp of user's last login
  pub last_login_at: Option<DateTime<Utc>>,
}

/// Use case for getting the current authenticated user
pub struct GetCurrentUserUseCase {
  auth_service: Arc<AuthService>,
}

impl GetCurrentUserUseCase {
  /// Creates a new instance of GetCurrentUserUseCase
  pub fn new(auth_service: Arc<AuthService>) -> Self {
    Self { auth_service }
  }

  /// Executes the get current user use case
  ///
  /// # Arguments
  /// * `session_token` - The session token to validate and retrieve user from
  ///
  /// # Returns
  /// A `GetCurrentUserResponse` containing the user's details
  ///
  /// # Errors
  /// Returns `AuthError` if the operation fails (e.g., invalid or expired session)
  pub async fn execute(&self, session_token: String) -> Result<GetCurrentUserResponse, AuthError> {
    // Parse and validate session token
    let token = SessionToken::from_string(session_token)?;

    // Validate session and get user using the auth service
    let user = self.auth_service.validate_session(token).await?;

    // Build and return the response
    // Note: We don't track last_login_at separately in the current User entity,
    // so we return None here. This could be enhanced by tracking login history.
    Ok(GetCurrentUserResponse {
      user_id: user.id,
      email: user.email,
      created_at: user.created_at,
      last_login_at: None,
    })
  }
}
