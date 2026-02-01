use thiserror::Error;

use super::value_objects::ValueObjectError;

/// Main authentication error type
#[derive(Debug, Error)]
pub enum AuthError {
  #[error("Invalid credentials provided")]
  InvalidCredentials,

  #[error("Email already exists")]
  EmailAlreadyExists,

  #[error("User not found")]
  UserNotFound,

  #[error("Invalid or expired session")]
  InvalidSession,

  #[error("Account has been deleted")]
  AccountDeleted,

  #[error("Rate limit exceeded, please try again later")]
  RateLimitExceeded,

  #[error("Repository error: {0}")]
  Repository(#[from] RepositoryError),

  #[error("Hash error: {0}")]
  Hash(#[from] HashError),

  #[error("Validation error: {0}")]
  Validation(#[from] ValidationError),

  #[error("Value object error: {0}")]
  ValueObject(#[from] ValueObjectError),
}

/// Repository-related errors
#[derive(Debug, Error)]
pub enum RepositoryError {
  #[error("Database connection failed: {0}")]
  ConnectionFailed(String),

  #[error("Query execution failed: {0}")]
  QueryFailed(String),

  #[error("Transaction failed: {0}")]
  TransactionFailed(String),

  #[error("Record not found")]
  NotFound,

  #[error("Duplicate key violation: {0}")]
  DuplicateKey(String),

  #[error("Database error: {0}")]
  DatabaseError(String),

  #[error("Invalid IP address: {0}")]
  InvalidIpAddress(#[from] std::net::AddrParseError),
}

/// Password hashing and verification errors
#[derive(Debug, Error)]
pub enum HashError {
  #[error("Failed to hash password: {0}")]
  HashingFailed(String),

  #[error("Failed to verify password: {0}")]
  VerificationFailed(String),

  #[error("Invalid hash format")]
  InvalidFormat,
}

/// Input validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
  #[error("Invalid email format")]
  InvalidEmail,

  #[error("Password too short, minimum {min} characters required")]
  PasswordTooShort { min: usize },

  #[error("Password too long, maximum {max} characters allowed")]
  PasswordTooLong { max: usize },

  #[error("Password must contain at least one uppercase letter")]
  PasswordMissingUppercase,

  #[error("Password must contain at least one lowercase letter")]
  PasswordMissingLowercase,

  #[error("Password must contain at least one digit")]
  PasswordMissingDigit,

  #[error("Password must contain at least one special character")]
  PasswordMissingSpecial,

  #[error("Invalid field: {field}")]
  InvalidField { field: String },

  #[error("Missing required field: {field}")]
  MissingField { field: String },
}

// Automatic conversions from external error types

impl From<sqlx::Error> for RepositoryError {
  fn from(error: sqlx::Error) -> Self {
    match error {
      sqlx::Error::RowNotFound => RepositoryError::NotFound,
      sqlx::Error::Database(db_err) => {
        if db_err.is_unique_violation() {
          RepositoryError::DuplicateKey(db_err.message().to_string())
        } else {
          RepositoryError::DatabaseError(db_err.message().to_string())
        }
      }
      sqlx::Error::PoolTimedOut => RepositoryError::ConnectionFailed("Pool timed out".to_string()),
      sqlx::Error::PoolClosed => RepositoryError::ConnectionFailed("Pool closed".to_string()),
      _ => RepositoryError::QueryFailed(error.to_string()),
    }
  }
}

impl From<sqlx::Error> for AuthError {
  fn from(error: sqlx::Error) -> Self {
    AuthError::Repository(RepositoryError::from(error))
  }
}

impl From<argon2::password_hash::Error> for ValueObjectError {
  fn from(error: argon2::password_hash::Error) -> Self {
    use argon2::password_hash::Error;
    match error {
      Error::Password => ValueObjectError::VerificationFailed("Invalid password".to_string()),
      // Hash parsing/format errors
      Error::PhcStringField | Error::PhcStringTrailingData => ValueObjectError::InvalidPasswordHash,
      // All other errors are hashing/verification failures
      _ => ValueObjectError::HashingFailed(error.to_string()),
    }
  }
}
