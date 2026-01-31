use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Request for user registration
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RegisterRequest {
  /// User's email address
  #[validate(email(message = "Invalid email format"))]
  pub email: String,

  /// User's password
  #[validate(length(
    min = 8,
    max = 128,
    message = "Password must be between 8 and 128 characters"
  ))]
  pub password: String,

  /// User's full name
  #[validate(length(
    min = 1,
    max = 255,
    message = "Full name must be between 1 and 255 characters"
  ))]
  pub full_name: String,
}

/// Request for user login
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct LoginRequest {
  /// User's email address
  #[validate(email(message = "Invalid email format"))]
  pub email: String,

  /// User's password
  #[validate(length(min = 1, message = "Password is required"))]
  pub password: String,

  /// Whether to create a long-lived session
  #[serde(default)]
  pub remember_me: bool,
}

/// Response after successful user registration
#[derive(Debug, Clone, Serialize)]
pub struct RegisterResponse {
  /// Unique identifier of the newly created user
  pub user_id: Uuid,

  /// User's email address
  pub email: String,

  /// Session token for authentication
  pub session_token: String,

  /// Session expiration timestamp
  pub expires_at: DateTime<Utc>,
}

/// Response after successful user login
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
  /// Unique identifier of the user
  pub user_id: Uuid,

  /// User's email address
  pub email: String,

  /// Session token for authentication
  pub session_token: String,

  /// Session expiration timestamp
  pub expires_at: DateTime<Utc>,

  /// Timestamp of user's last login (before this one)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_login_at: Option<DateTime<Utc>>,
}

/// Response after successful logout from all devices
#[derive(Debug, Clone, Serialize)]
pub struct LogoutAllResponse {
  /// Number of sessions that were terminated
  pub sessions_terminated: usize,

  /// Success message
  pub message: String,
}

/// Response containing current user information
#[derive(Debug, Clone, Serialize)]
pub struct CurrentUserResponse {
  /// Unique identifier of the user
  pub user_id: Uuid,

  /// User's email address
  pub email: String,

  /// Timestamp when the user account was created
  pub created_at: DateTime<Utc>,

  /// Timestamp of user's last login
  #[serde(skip_serializing_if = "Option::is_none")]
  pub last_login_at: Option<DateTime<Utc>>,
}

/// Standard success response for operations without data
#[derive(Debug, Clone, Serialize)]
pub struct SuccessResponse {
  /// Success message
  pub message: String,
}

/// Standard error response
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
  /// Error type/code
  pub error: String,

  /// Human-readable error message
  pub message: String,

  /// Optional detailed error information
  #[serde(skip_serializing_if = "Option::is_none")]
  pub details: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use validator::Validate;

  #[test]
  fn test_register_request_validation_valid() {
    let request = RegisterRequest {
      email: "test@example.com".to_string(),
      password: "SecureP@ss123".to_string(),
      full_name: "Test User".to_string(),
    };

    assert!(request.validate().is_ok());
  }

  #[test]
  fn test_register_request_validation_invalid_email() {
    let request = RegisterRequest {
      email: "invalid-email".to_string(),
      password: "SecureP@ss123".to_string(),
      full_name: "Test User".to_string(),
    };

    assert!(request.validate().is_err());
  }

  #[test]
  fn test_register_request_validation_short_password() {
    let request = RegisterRequest {
      email: "test@example.com".to_string(),
      password: "short".to_string(),
      full_name: "Test User".to_string(),
    };

    assert!(request.validate().is_err());
  }

  #[test]
  fn test_login_request_validation_valid() {
    let request = LoginRequest {
      email: "test@example.com".to_string(),
      password: "password123".to_string(),
      remember_me: false,
    };

    assert!(request.validate().is_ok());
  }

  #[test]
  fn test_login_request_remember_me_default() {
    let json = r#"{"email": "test@example.com", "password": "password"}"#;
    let request: LoginRequest = serde_json::from_str(json).unwrap();

    assert!(!request.remember_me);
  }
}
