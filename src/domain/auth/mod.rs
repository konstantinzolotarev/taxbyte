pub mod entities;
pub mod errors;
pub mod ports;
pub mod services;
pub mod value_objects;

// Re-export commonly used types
pub use entities::{LoginAttempt, Session, User};
pub use errors::{AuthError, HashError, RepositoryError, ValidationError};
pub use value_objects::{
  Email, FailureReason, Password, PasswordHash, SessionId, SessionToken, TokenHash, UserId,
};
