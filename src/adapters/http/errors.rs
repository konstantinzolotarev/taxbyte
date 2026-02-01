use actix_web::{
  HttpResponse,
  error::ResponseError,
  http::{StatusCode, header::ContentType},
};
use serde::Serialize;
use std::fmt;

use crate::domain::auth::errors::{AuthError, RepositoryError};
use crate::domain::company::CompanyError;

use super::dtos::ErrorResponse;

/// API error type that maps domain errors to HTTP responses
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum ApiError {
  /// Validation error (400 Bad Request)
  Validation(String),

  /// Authentication error (401 Unauthorized or 403 Forbidden)
  Auth(AuthErrorKind),

  /// Internal server error (500 Internal Server Error)
  Internal(String),
}

/// Authentication error kinds
#[derive(Debug, Serialize)]
pub enum AuthErrorKind {
  /// Invalid credentials (401)
  InvalidCredentials,

  /// Session expired or invalid (401)
  InvalidSession,

  /// Invalid token format (401)
  InvalidToken,

  /// Rate limit exceeded (429)
  RateLimitExceeded,

  /// Email already exists (409)
  EmailAlreadyExists,

  /// User not found (404)
  UserNotFound,

  /// Account deleted (403)
  AccountDeleted,
}

impl fmt::Display for ApiError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ApiError::Validation(msg) => write!(f, "Validation error: {}", msg),
      ApiError::Auth(kind) => write!(f, "Authentication error: {:?}", kind),
      ApiError::Internal(msg) => write!(f, "Internal error: {}", msg),
    }
  }
}

impl ResponseError for ApiError {
  fn status_code(&self) -> StatusCode {
    match self {
      ApiError::Validation(_) => StatusCode::BAD_REQUEST,
      ApiError::Auth(kind) => match kind {
        AuthErrorKind::InvalidCredentials => StatusCode::UNAUTHORIZED,
        AuthErrorKind::InvalidSession => StatusCode::UNAUTHORIZED,
        AuthErrorKind::InvalidToken => StatusCode::UNAUTHORIZED,
        AuthErrorKind::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
        AuthErrorKind::EmailAlreadyExists => StatusCode::CONFLICT,
        AuthErrorKind::UserNotFound => StatusCode::NOT_FOUND,
        AuthErrorKind::AccountDeleted => StatusCode::FORBIDDEN,
      },
      ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }

  fn error_response(&self) -> HttpResponse {
    let status = self.status_code();
    let (error_type, message, details) = match self {
      ApiError::Validation(msg) => ("validation_error", msg.clone(), None),
      ApiError::Auth(kind) => {
        let (err_type, msg) = match kind {
          AuthErrorKind::InvalidCredentials => (
            "invalid_credentials",
            "Invalid email or password".to_string(),
          ),
          AuthErrorKind::InvalidSession => {
            ("invalid_session", "Invalid or expired session".to_string())
          }
          AuthErrorKind::InvalidToken => (
            "invalid_token",
            "Invalid or missing authorization token".to_string(),
          ),
          AuthErrorKind::RateLimitExceeded => (
            "rate_limit_exceeded",
            "Too many login attempts. Please try again later".to_string(),
          ),
          AuthErrorKind::EmailAlreadyExists => (
            "email_already_exists",
            "An account with this email already exists".to_string(),
          ),
          AuthErrorKind::UserNotFound => ("user_not_found", "User not found".to_string()),
          AuthErrorKind::AccountDeleted => (
            "account_deleted",
            "This account has been deleted".to_string(),
          ),
        };
        (err_type, msg, None)
      }
      ApiError::Internal(msg) => {
        // Don't expose internal error details in production
        tracing::error!("Internal error: {}", msg);
        (
          "internal_error",
          "An internal server error occurred".to_string(),
          None,
        )
      }
    };

    let error_response = ErrorResponse {
      error: error_type.to_string(),
      message,
      details,
    };

    HttpResponse::build(status)
      .content_type(ContentType::json())
      .json(error_response)
  }
}

/// Convert AuthError to ApiError
impl From<AuthError> for ApiError {
  fn from(error: AuthError) -> Self {
    match error {
      AuthError::InvalidCredentials => ApiError::Auth(AuthErrorKind::InvalidCredentials),
      AuthError::EmailAlreadyExists => ApiError::Auth(AuthErrorKind::EmailAlreadyExists),
      AuthError::UserNotFound => ApiError::Auth(AuthErrorKind::UserNotFound),
      AuthError::InvalidSession => ApiError::Auth(AuthErrorKind::InvalidSession),
      AuthError::AccountDeleted => ApiError::Auth(AuthErrorKind::AccountDeleted),
      AuthError::RateLimitExceeded => ApiError::Auth(AuthErrorKind::RateLimitExceeded),
      AuthError::Validation(err) => ApiError::Validation(err.to_string()),
      AuthError::ValueObject(err) => ApiError::Validation(err.to_string()),
      AuthError::Repository(err) => match err {
        RepositoryError::NotFound => ApiError::Auth(AuthErrorKind::UserNotFound),
        RepositoryError::DuplicateKey(_) => ApiError::Auth(AuthErrorKind::EmailAlreadyExists),
        _ => ApiError::Internal(err.to_string()),
      },
      AuthError::Hash(err) => ApiError::Internal(err.to_string()),
    }
  }
}

/// Convert validation errors from validator crate
impl From<validator::ValidationErrors> for ApiError {
  fn from(errors: validator::ValidationErrors) -> Self {
    let messages: Vec<String> = errors
      .field_errors()
      .iter()
      .flat_map(|(field, errors)| {
        errors
          .iter()
          .map(|error| {
            error
              .message
              .as_ref()
              .map(|m| m.to_string())
              .unwrap_or_else(|| format!("Invalid field: {}", field))
          })
          .collect::<Vec<_>>()
      })
      .collect();

    ApiError::Validation(messages.join(", "))
  }
}

/// Convert CompanyError to ApiError
impl From<CompanyError> for ApiError {
  fn from(error: CompanyError) -> Self {
    match error {
      CompanyError::NotFound => ApiError::Validation("Company not found".to_string()),
      CompanyError::NotMember => ApiError::Auth(AuthErrorKind::InvalidSession),
      CompanyError::AlreadyMember => ApiError::Validation("User is already a member".to_string()),
      CompanyError::InsufficientPermissions => ApiError::Auth(AuthErrorKind::AccountDeleted),
      CompanyError::CannotRemoveLastOwner => {
        ApiError::Validation("Cannot remove the last owner".to_string())
      }
      CompanyError::UserNotFound => ApiError::Auth(AuthErrorKind::UserNotFound),
      CompanyError::Repository(e) => ApiError::Internal(format!("Repository error: {}", e)),
      CompanyError::Validation(e) => ApiError::Validation(e.to_string()),
      CompanyError::Auth(e) => ApiError::from(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_api_error_status_codes() {
    assert_eq!(
      ApiError::Validation("test".to_string()).status_code(),
      StatusCode::BAD_REQUEST
    );
    assert_eq!(
      ApiError::Auth(AuthErrorKind::InvalidCredentials).status_code(),
      StatusCode::UNAUTHORIZED
    );
    assert_eq!(
      ApiError::Auth(AuthErrorKind::EmailAlreadyExists).status_code(),
      StatusCode::CONFLICT
    );
    assert_eq!(
      ApiError::Internal("test".to_string()).status_code(),
      StatusCode::INTERNAL_SERVER_ERROR
    );
  }

  #[test]
  fn test_auth_error_conversion() {
    let api_error: ApiError = AuthError::InvalidCredentials.into();
    assert_eq!(api_error.status_code(), StatusCode::UNAUTHORIZED);

    let api_error: ApiError = AuthError::EmailAlreadyExists.into();
    assert_eq!(api_error.status_code(), StatusCode::CONFLICT);
  }
}
