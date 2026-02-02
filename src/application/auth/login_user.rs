use chrono::{DateTime, Utc};
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::auth::errors::AuthError;
use crate::domain::auth::services::AuthService;
use crate::domain::auth::value_objects::{Email, Password};

/// Command for logging in a user
#[derive(Debug, Clone)]
pub struct LoginUserCommand {
  /// User's email address
  pub email: String,
  /// User's password (plain text)
  pub password: String,
  /// Whether to create a long-lived session
  pub remember_me: bool,
}

/// Response after successful user login
#[derive(Debug, Clone)]
pub struct LoginUserResponse {
  /// Unique identifier of the user
  pub user_id: Uuid,
  /// User's email address
  pub email: String,
  /// Session token for authentication
  pub session_token: String,
  /// Session expiration timestamp
  pub expires_at: DateTime<Utc>,
  /// Timestamp of user's last login (before this one)
  pub last_login_at: Option<DateTime<Utc>>,
}

/// Use case for logging in a user
pub struct LoginUserUseCase {
  auth_service: Arc<AuthService>,
}

impl LoginUserUseCase {
  /// Creates a new instance of LoginUserUseCase
  pub fn new(auth_service: Arc<AuthService>) -> Self {
    Self { auth_service }
  }

  /// Executes the user login use case
  ///
  /// # Arguments
  /// * `command` - The login command containing credentials
  /// * `ip_address` - Optional IP address of the client
  /// * `user_agent` - Optional user agent string from the client
  ///
  /// # Returns
  /// A `LoginUserResponse` containing the user's details and session token
  ///
  /// # Errors
  /// Returns `AuthError` if login fails (e.g., invalid credentials, rate limit exceeded)
  pub async fn execute(
    &self,
    command: LoginUserCommand,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
  ) -> Result<LoginUserResponse, AuthError> {
    // Parse and validate email
    let email = Email::new(command.email)?;

    // Parse and validate password
    let password = Password::new(command.password)?;

    // Authenticate the user using the auth service
    let (user, session, session_token) = self
      .auth_service
      .login(email, password, ip_address, user_agent, command.remember_me)
      .await?;

    // Build and return the response
    // Note: We don't track last_login_at separately in the current User entity,
    // so we return None here. This could be enhanced by tracking login history.
    Ok(LoginUserResponse {
      user_id: user.id,
      email: user.email,
      session_token: session_token.into_inner(),
      expires_at: session.expires_at,
      last_login_at: None,
    })
  }
}

#[cfg(test)]
mod tests {

  #[tokio::test]
  async fn test_placeholder() {
    // This test would require mock implementations of AuthService
    // For now, this is just a placeholder to show the test structure
  }
}
