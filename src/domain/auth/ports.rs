use async_trait::async_trait;
use uuid::Uuid;

use super::entities::{LoginAttempt, Session, User};
use super::errors::AuthError;
use super::value_objects::{Email, Password, PasswordHash};

/// Repository trait for user persistence operations
#[async_trait]
pub trait UserRepository: Send + Sync {
  /// Creates a new user in the repository
  async fn create(&self, user: User) -> Result<User, AuthError>;

  /// Finds a user by their unique identifier
  async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AuthError>;

  /// Finds a user by their email address
  async fn find_by_email(&self, email: &Email) -> Result<Option<User>, AuthError>;

  /// Updates an existing user
  async fn update(&self, user: User) -> Result<User, AuthError>;

  /// Soft deletes a user (marks as deleted without removing from database)
  async fn soft_delete(&self, id: Uuid) -> Result<(), AuthError>;
}

/// Repository trait for session persistence operations
#[async_trait]
pub trait SessionRepository: Send + Sync {
  /// Creates a new session in the repository
  async fn create(&self, session: Session) -> Result<Session, AuthError>;

  /// Finds a session by its token hash
  async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, AuthError>;

  /// Finds all active sessions for a specific user
  async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Session>, AuthError>;

  /// Updates the last activity timestamp for a session
  async fn update_activity(&self, session_id: Uuid) -> Result<(), AuthError>;

  /// Deletes a specific session
  async fn delete(&self, session_id: Uuid) -> Result<(), AuthError>;

  /// Deletes all sessions for a specific user
  async fn delete_all_for_user(&self, user_id: Uuid) -> Result<(), AuthError>;
}

/// Repository trait for login attempt tracking operations
#[async_trait]
pub trait LoginAttemptRepository: Send + Sync {
  /// Records a new login attempt
  async fn create(&self, attempt: LoginAttempt) -> Result<LoginAttempt, AuthError>;

  /// Counts the number of recent failed login attempts for a user
  /// within a specified time window (in seconds)
  async fn count_recent_failures(
    &self,
    user_id: Uuid,
    window_seconds: i64,
  ) -> Result<i64, AuthError>;
}

/// Service trait for password hashing operations
#[async_trait]
pub trait PasswordHasher: Send + Sync {
  /// Hashes a plain text password
  async fn hash(&self, password: &Password) -> Result<PasswordHash, AuthError>;

  /// Verifies a plain text password against a hashed password
  async fn verify(
    &self,
    password: &Password,
    hashed_password: &PasswordHash,
  ) -> Result<bool, AuthError>;
}

/// Service trait for secure token generation
#[async_trait]
pub trait TokenGenerator: Send + Sync {
  /// Generates a cryptographically secure random token
  async fn generate(&self) -> Result<String, AuthError>;
}
