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

/// Configuration for AuthService
#[derive(Debug, Clone)]
pub struct AuthServiceConfig {
  /// Session duration in seconds (e.g., 3600 for 1 hour)
  pub session_ttl_seconds: i64,
  /// Remember me duration in seconds (e.g., 2592000 for 30 days)
  pub remember_me_ttl_seconds: i64,
  /// Rate limit window in seconds (e.g., 300 for 5 minutes)
  pub rate_limit_window_seconds: i64,
  /// Maximum failed login attempts before rate limiting
  pub max_failed_attempts: i64,
}

/// Authentication service implementing core business logic
pub struct AuthService {
  user_repo: Arc<dyn UserRepository>,
  session_repo: Arc<dyn SessionRepository>,
  attempt_repo: Arc<dyn LoginAttemptRepository>,
  password_hasher: Arc<dyn PasswordHasher>,
  #[allow(dead_code)] // Reserved for future token refresh functionality
  token_generator: Arc<dyn TokenGenerator>,
  // Configuration values
  session_duration: Duration,
  remember_me_duration: Duration,
  rate_limit_window: Duration,
  max_failed_attempts: i64,
}

impl AuthService {
  /// Creates a new instance of AuthService
  ///
  /// # Arguments
  /// * `user_repo` - User repository
  /// * `session_repo` - Session repository
  /// * `attempt_repo` - Login attempt repository
  /// * `password_hasher` - Password hasher implementation
  /// * `token_generator` - Token generator implementation
  /// * `config` - Service configuration (TTLs, rate limits, etc.)
  pub fn new(
    user_repo: Arc<dyn UserRepository>,
    session_repo: Arc<dyn SessionRepository>,
    attempt_repo: Arc<dyn LoginAttemptRepository>,
    password_hasher: Arc<dyn PasswordHasher>,
    token_generator: Arc<dyn TokenGenerator>,
    config: AuthServiceConfig,
  ) -> Self {
    Self {
      user_repo,
      session_repo,
      attempt_repo,
      password_hasher,
      token_generator,
      session_duration: Duration::seconds(config.session_ttl_seconds),
      remember_me_duration: Duration::seconds(config.remember_me_ttl_seconds),
      rate_limit_window: Duration::seconds(config.rate_limit_window_seconds),
      max_failed_attempts: config.max_failed_attempts,
    }
  }

  /// Helper method to generate a session token with proper error mapping
  fn generate_session_token() -> Result<SessionToken, AuthError> {
    SessionToken::generate().map_err(|e| AuthError::invalid_field(format!("session_token: {}", e)))
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
    let session_token = Self::generate_session_token()?;
    let token_hash = session_token.hash();

    // Create session with default duration
    let session = Session::with_duration(
      created_user.id,
      token_hash.into_inner(),
      self.session_duration,
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
      let window_seconds = self.rate_limit_window.num_seconds();
      let failed_attempts = self
        .attempt_repo
        .count_recent_failures(user.id, window_seconds)
        .await?;

      if failed_attempts >= self.max_failed_attempts {
        // Record failed attempt
        let attempt = LoginAttempt::failure(email.into_inner(), ip);
        self.attempt_repo.create(attempt).await?;

        return Err(AuthError::RateLimitExceeded);
      }
    }

    // Verify password
    let password_hash = super::value_objects::PasswordHash::from_hash(&user.password_hash)
      .map_err(|e| AuthError::invalid_field(format!("password_hash: {}", e)))?;

    let is_valid = self
      .password_hasher
      .verify(&password, &password_hash)
      .await?;

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
    let session_token = Self::generate_session_token()?;
    let token_hash = session_token.hash();

    // Determine session duration based on remember_me flag
    let duration = if remember_me {
      self.remember_me_duration
    } else {
      self.session_duration
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
