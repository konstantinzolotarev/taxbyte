use chrono::Duration;
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

use super::entities::{LoginAttempt, Session, User};
use super::errors::{AuthError, RepositoryError};
use super::ports::{
  LoginAttemptRepository, PasswordHasher, SessionRepository, TokenGenerator, UserRepository,
};
use super::value_objects::{Email, Password, SessionToken};

/// Configuration constants for authentication
const SESSION_DURATION_HOURS: i64 = 24;
const REMEMBER_ME_DURATION_DAYS: i64 = 30;
const RATE_LIMIT_WINDOW_MINUTES: i64 = 15;
const MAX_FAILED_ATTEMPTS: i64 = 5;

/// Authentication service implementing core business logic
pub struct AuthService {
  user_repo: Arc<dyn UserRepository>,
  session_repo: Arc<dyn SessionRepository>,
  attempt_repo: Arc<dyn LoginAttemptRepository>,
  password_hasher: Arc<dyn PasswordHasher>,
  #[allow(dead_code)] // Reserved for future token refresh functionality
  token_generator: Arc<dyn TokenGenerator>,
}

impl AuthService {
  /// Creates a new instance of AuthService
  pub fn new(
    user_repo: Arc<dyn UserRepository>,
    session_repo: Arc<dyn SessionRepository>,
    attempt_repo: Arc<dyn LoginAttemptRepository>,
    password_hasher: Arc<dyn PasswordHasher>,
    token_generator: Arc<dyn TokenGenerator>,
  ) -> Self {
    Self {
      user_repo,
      session_repo,
      attempt_repo,
      password_hasher,
      token_generator,
    }
  }

  /// Registers a new user with email and password
  ///
  /// # Arguments
  /// * `email` - The user's email address
  /// * `password` - The user's password (will be hashed)
  /// * `full_name` - The user's full name
  ///
  /// # Returns
  /// A tuple containing (User, Session, SessionToken) on success
  ///
  /// # Errors
  /// Returns `AuthError::EmailAlreadyExists` if email is already registered
  pub async fn register(
    &self,
    email: Email,
    password: Password,
    full_name: String,
  ) -> Result<(User, Session, SessionToken), AuthError> {
    // Check if email already exists
    if let Some(_existing_user) = self.user_repo.find_by_email(&email).await? {
      return Err(AuthError::EmailAlreadyExists);
    }

    // Hash the password
    let password_hash = self.password_hasher.hash(&password).await?;

    // Create new user
    let user = User::new(email.into_inner(), password_hash.into_inner(), full_name);

    // Save user to repository
    let created_user = match self.user_repo.create(user).await {
      Ok(user) => user,
      Err(AuthError::Repository(RepositoryError::DuplicateKey(_))) => {
        return Err(AuthError::EmailAlreadyExists);
      }
      Err(e) => return Err(e),
    };

    // Generate session token
    let session_token = SessionToken::generate().map_err(|e| {
      AuthError::Validation(super::errors::ValidationError::InvalidField {
        field: format!("session_token: {}", e),
      })
    })?;

    let token_hash = session_token.hash();

    // Create session with default duration
    let session = Session::with_duration(
      created_user.id,
      token_hash.into_inner(),
      Duration::hours(SESSION_DURATION_HOURS),
      None,
      None,
    );

    // Save session to repository
    let created_session = self.session_repo.create(session).await?;

    Ok((created_user, created_session, session_token))
  }

  /// Authenticates a user and creates a new session
  ///
  /// # Arguments
  /// * `email` - The user's email address
  /// * `password` - The user's password
  /// * `ip_address` - Optional IP address of the client
  /// * `user_agent` - Optional user agent string
  /// * `remember_me` - Whether to create a long-lived session
  ///
  /// # Returns
  /// A tuple containing (User, Session, SessionToken) on success
  ///
  /// # Errors
  /// Returns various AuthError variants for different failure scenarios
  pub async fn login(
    &self,
    email: Email,
    password: Password,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
    remember_me: bool,
  ) -> Result<(User, Session, SessionToken), AuthError> {
    // Find user by email
    let user = self
      .user_repo
      .find_by_email(&email)
      .await?
      .ok_or(AuthError::InvalidCredentials)?;

    // Check rate limiting - count recent failed attempts
    if let Some(ip) = ip_address {
      let window_seconds = RATE_LIMIT_WINDOW_MINUTES * 60;
      let failed_attempts = self
        .attempt_repo
        .count_recent_failures(user.id, window_seconds)
        .await?;

      if failed_attempts >= MAX_FAILED_ATTEMPTS {
        // Record failed attempt
        let attempt = LoginAttempt::failure(email.into_inner(), ip);
        self.attempt_repo.create(attempt).await?;

        return Err(AuthError::RateLimitExceeded);
      }
    }

    // Verify password
    let password_hash = super::value_objects::PasswordHash::from_hash(&user.password_hash)
      .map_err(|e| {
        AuthError::Validation(super::errors::ValidationError::InvalidField {
          field: format!("password_hash: {}", e),
        })
      })?;

    let is_valid = password_hash.verify(&password).map_err(|e| {
      AuthError::Validation(super::errors::ValidationError::InvalidField {
        field: format!("password_verification: {}", e),
      })
    })?;

    if !is_valid {
      // Record failed attempt
      if let Some(ip) = ip_address {
        let attempt = LoginAttempt::failure(email.into_inner(), ip);
        self.attempt_repo.create(attempt).await?;
      }

      return Err(AuthError::InvalidCredentials);
    }

    // Record successful attempt
    if let Some(ip) = ip_address {
      let attempt = LoginAttempt::success(email.into_inner(), ip);
      self.attempt_repo.create(attempt).await?;
    }

    // Generate session token
    let session_token = SessionToken::generate().map_err(|e| {
      AuthError::Validation(super::errors::ValidationError::InvalidField {
        field: format!("session_token: {}", e),
      })
    })?;

    let token_hash = session_token.hash();

    // Determine session duration based on remember_me flag
    let duration = if remember_me {
      Duration::days(REMEMBER_ME_DURATION_DAYS)
    } else {
      Duration::hours(SESSION_DURATION_HOURS)
    };

    // Create session
    let session = Session::with_duration(
      user.id,
      token_hash.into_inner(),
      duration,
      ip_address,
      user_agent,
    );

    // Save session to repository
    let created_session = self.session_repo.create(session).await?;

    Ok((user, created_session, session_token))
  }

  /// Logs out a user by invalidating their session token
  ///
  /// # Arguments
  /// * `token` - The session token to invalidate
  ///
  /// # Returns
  /// Ok(()) on success
  ///
  /// # Errors
  /// Returns `AuthError::InvalidSession` if session not found
  pub async fn logout(&self, token: SessionToken) -> Result<(), AuthError> {
    let token_hash = token.hash();

    // Find session by token hash
    let session = self
      .session_repo
      .find_by_token_hash(token_hash.as_str())
      .await?
      .ok_or(AuthError::InvalidSession)?;

    // Delete the session
    self.session_repo.delete(session.id).await?;

    Ok(())
  }

  /// Logs out all sessions for a specific user
  ///
  /// # Arguments
  /// * `user_id` - The ID of the user whose sessions should be invalidated
  ///
  /// # Returns
  /// The number of sessions deleted
  ///
  /// # Errors
  /// Returns `AuthError::UserNotFound` if user doesn't exist
  pub async fn logout_all(&self, user_id: Uuid) -> Result<usize, AuthError> {
    // Verify user exists
    self
      .user_repo
      .find_by_id(user_id)
      .await?
      .ok_or(AuthError::UserNotFound)?;

    // Get all sessions for the user before deleting
    let sessions = self.session_repo.find_by_user_id(user_id).await?;
    let session_count = sessions.len();

    // Delete all sessions for the user
    self.session_repo.delete_all_for_user(user_id).await?;

    Ok(session_count)
  }

  /// Validates a session token and returns the associated user
  ///
  /// # Arguments
  /// * `token` - The session token to validate
  ///
  /// # Returns
  /// The User associated with the session
  ///
  /// # Errors
  /// Returns `AuthError::InvalidSession` if session is invalid or expired
  pub async fn validate_session(&self, token: SessionToken) -> Result<User, AuthError> {
    let token_hash = token.hash();

    // Find session by token hash
    let session = self
      .session_repo
      .find_by_token_hash(token_hash.as_str())
      .await?
      .ok_or(AuthError::InvalidSession)?;

    // Check if session is expired
    if session.is_expired() {
      // Delete expired session
      self.session_repo.delete(session.id).await?;
      return Err(AuthError::InvalidSession);
    }

    // Find and return the user
    let user = self
      .user_repo
      .find_by_id(session.user_id)
      .await?
      .ok_or(AuthError::UserNotFound)?;

    // Update session activity timestamp
    self.session_repo.update_activity(session.id).await?;

    Ok(user)
  }
}

#[cfg(test)]
mod tests {
  // Mock implementations would go here for testing
  // This is a placeholder for the test structure

  #[tokio::test]
  async fn test_placeholder() {
    // This test would require mock implementations of all the traits
    // For now, this is just a placeholder to show the test structure
  }
}
