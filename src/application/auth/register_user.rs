use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::auth::errors::AuthError;
use crate::domain::auth::services::AuthService;
use crate::domain::auth::value_objects::{Email, Password};

/// Command for registering a new user
#[derive(Debug, Clone)]
pub struct RegisterUserCommand {
  /// User's email address
  pub email: String,
  /// User's password (plain text, will be hashed)
  pub password: String,
  /// User's full name
  pub full_name: String,
}

/// Response after successful user registration
#[derive(Debug, Clone)]
pub struct RegisterUserResponse {
  /// Unique identifier of the newly created user
  pub user_id: Uuid,
  /// User's email address
  pub email: String,
  /// Session token for immediate login
  pub session_token: String,
  /// Session expiration timestamp
  pub expires_at: DateTime<Utc>,
}

/// Use case for registering a new user
pub struct RegisterUserUseCase {
  auth_service: Arc<AuthService>,
}

impl RegisterUserUseCase {
  /// Creates a new instance of RegisterUserUseCase
  pub fn new(auth_service: Arc<AuthService>) -> Self {
    Self { auth_service }
  }

  /// Executes the user registration use case
  ///
  /// # Arguments
  /// * `command` - The registration command containing user details
  ///
  /// # Returns
  /// A `RegisterUserResponse` containing the new user's details and session token
  ///
  /// # Errors
  /// Returns `AuthError` if registration fails (e.g., email already exists, validation errors)
  pub async fn execute(
    &self,
    command: RegisterUserCommand,
  ) -> Result<RegisterUserResponse, AuthError> {
    // Parse and validate email
    let email = Email::new(command.email)?;

    // Parse and validate password
    let password = Password::new(command.password)?;

    // Register the user using the auth service
    let (user, session, session_token) = self
      .auth_service
      .register(email, password, command.full_name)
      .await?;

    // Build and return the response
    Ok(RegisterUserResponse {
      user_id: user.id,
      email: user.email,
      session_token: session_token.into_inner(),
      expires_at: session.expires_at,
    })
  }
}

#[cfg(test)]
mod tests {

  #[tokio::test]
  async fn test_placeholder() {
    // This test would require mock implementations of AuthService
    // For now, this is just a placeholder to show the test structure
    assert!(true);
  }
}
