use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

/// User entity representing a user in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
  /// Unique identifier for the user
  pub id: Uuid,
  /// User's email address (unique)
  pub email: String,
  /// Hashed password using Argon2
  pub password_hash: String,
  /// User's full name
  pub full_name: String,
  /// Whether the user's email has been verified
  pub is_email_verified: bool,
  /// Token for email verification (optional)
  pub email_verification_token: Option<String>,
  /// Expiration time for email verification token
  pub email_verification_token_expires_at: Option<DateTime<Utc>>,
  /// Token for password reset (optional)
  pub password_reset_token: Option<String>,
  /// Expiration time for password reset token
  pub password_reset_token_expires_at: Option<DateTime<Utc>>,
  /// Timestamp when the user was created
  pub created_at: DateTime<Utc>,
  /// Timestamp when the user was last updated
  pub updated_at: DateTime<Utc>,
}

impl User {
  /// Creates a new user with the given details
  pub fn new(email: String, password_hash: String, full_name: String) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4(),
      email,
      password_hash,
      full_name,
      is_email_verified: false,
      email_verification_token: None,
      email_verification_token_expires_at: None,
      password_reset_token: None,
      password_reset_token_expires_at: None,
      created_at: now,
      updated_at: now,
    }
  }

  /// Creates a user from database fields (for reconstruction)
  #[allow(clippy::too_many_arguments)]
  pub fn from_db(
    id: Uuid,
    email: String,
    password_hash: String,
    full_name: String,
    is_email_verified: bool,
    email_verification_token: Option<String>,
    email_verification_token_expires_at: Option<DateTime<Utc>>,
    password_reset_token: Option<String>,
    password_reset_token_expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
  ) -> Self {
    Self {
      id,
      email,
      password_hash,
      full_name,
      is_email_verified,
      email_verification_token,
      email_verification_token_expires_at,
      password_reset_token,
      password_reset_token_expires_at,
      created_at,
      updated_at,
    }
  }

  /// Updates the user's password hash
  pub fn update_password(&mut self, new_password_hash: String) {
    self.password_hash = new_password_hash;
    self.updated_at = Utc::now();
  }

  /// Marks the email as verified and clears the verification token
  pub fn verify_email(&mut self) {
    self.is_email_verified = true;
    self.email_verification_token = None;
    self.email_verification_token_expires_at = None;
    self.updated_at = Utc::now();
  }

  /// Sets a new email verification token with expiration
  pub fn set_email_verification_token(&mut self, token: String, expires_in: Duration) {
    self.email_verification_token = Some(token);
    self.email_verification_token_expires_at = Some(Utc::now() + expires_in);
    self.updated_at = Utc::now();
  }

  /// Checks if the email verification token is valid and not expired
  pub fn is_email_verification_token_valid(&self, token: &str) -> bool {
    match (
      &self.email_verification_token,
      &self.email_verification_token_expires_at,
    ) {
      (Some(stored_token), Some(expires_at)) => stored_token == token && expires_at > &Utc::now(),
      _ => false,
    }
  }

  /// Sets a new password reset token with expiration
  pub fn set_password_reset_token(&mut self, token: String, expires_in: Duration) {
    self.password_reset_token = Some(token);
    self.password_reset_token_expires_at = Some(Utc::now() + expires_in);
    self.updated_at = Utc::now();
  }

  /// Checks if the password reset token is valid and not expired
  pub fn is_password_reset_token_valid(&self, token: &str) -> bool {
    match (
      &self.password_reset_token,
      &self.password_reset_token_expires_at,
    ) {
      (Some(stored_token), Some(expires_at)) => stored_token == token && expires_at > &Utc::now(),
      _ => false,
    }
  }

  /// Clears the password reset token after successful password reset
  pub fn clear_password_reset_token(&mut self) {
    self.password_reset_token = None;
    self.password_reset_token_expires_at = None;
    self.updated_at = Utc::now();
  }

  /// Updates the user's email
  pub fn update_email(&mut self, new_email: String) {
    self.email = new_email;
    self.is_email_verified = false; // Require re-verification
    self.updated_at = Utc::now();
  }

  /// Updates the user's full name
  pub fn update_full_name(&mut self, new_full_name: String) {
    self.full_name = new_full_name;
    self.updated_at = Utc::now();
  }
}

/// Session entity representing an active user session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
  /// Unique identifier for the session
  pub id: Uuid,
  /// Reference to the user who owns this session
  pub user_id: Uuid,
  /// Session token (JWT or similar)
  pub session_token: String,
  /// IP address from which the session was created
  pub ip_address: Option<IpAddr>,
  /// User agent string from the client
  pub user_agent: Option<String>,
  /// Timestamp when the session expires
  pub expires_at: DateTime<Utc>,
  /// Timestamp when the session was created
  pub created_at: DateTime<Utc>,
}

impl Session {
  /// Creates a new session for a user
  pub fn new(
    user_id: Uuid,
    session_token: String,
    expires_at: DateTime<Utc>,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
  ) -> Self {
    Self {
      id: Uuid::new_v4(),
      user_id,
      session_token,
      ip_address,
      user_agent,
      expires_at,
      created_at: Utc::now(),
    }
  }

  /// Creates a session with a duration instead of absolute expiration time
  pub fn with_duration(
    user_id: Uuid,
    session_token: String,
    duration: Duration,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
  ) -> Self {
    let expires_at = Utc::now() + duration;
    Self::new(user_id, session_token, expires_at, ip_address, user_agent)
  }

  /// Creates a session from database fields (for reconstruction)
  pub fn from_db(
    id: Uuid,
    user_id: Uuid,
    session_token: String,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
  ) -> Self {
    Self {
      id,
      user_id,
      session_token,
      ip_address,
      user_agent,
      expires_at,
      created_at,
    }
  }

  /// Checks if the session has expired
  pub fn is_expired(&self) -> bool {
    self.expires_at <= Utc::now()
  }

  /// Checks if the session is still valid (not expired)
  pub fn is_valid(&self) -> bool {
    !self.is_expired()
  }

  /// Returns the remaining time until expiration
  pub fn time_until_expiration(&self) -> Duration {
    self.expires_at - Utc::now()
  }

  /// Extends the session expiration by the given duration
  pub fn extend(&mut self, duration: Duration) {
    self.expires_at += duration;
  }

  /// Refreshes the session with a new expiration time
  pub fn refresh(&mut self, new_expires_at: DateTime<Utc>) {
    self.expires_at = new_expires_at;
  }

  /// Refreshes the session with a duration from now
  pub fn refresh_with_duration(&mut self, duration: Duration) {
    self.expires_at = Utc::now() + duration;
  }
}

/// LoginAttempt entity for tracking authentication attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginAttempt {
  /// Unique identifier for the login attempt
  pub id: Uuid,
  /// Email address used in the login attempt
  pub email: String,
  /// IP address from which the attempt was made
  pub ip_address: IpAddr,
  /// Whether the login attempt was successful
  pub success: bool,
  /// Timestamp when the attempt was made
  pub attempted_at: DateTime<Utc>,
}

impl LoginAttempt {
  /// Creates a new login attempt record
  pub fn new(email: String, ip_address: IpAddr, success: bool) -> Self {
    Self {
      id: Uuid::new_v4(),
      email,
      ip_address,
      success,
      attempted_at: Utc::now(),
    }
  }

  /// Creates a successful login attempt
  pub fn success(email: String, ip_address: IpAddr) -> Self {
    Self::new(email, ip_address, true)
  }

  /// Creates a failed login attempt
  pub fn failure(email: String, ip_address: IpAddr) -> Self {
    Self::new(email, ip_address, false)
  }

  /// Creates a login attempt from database fields (for reconstruction)
  pub fn from_db(
    id: Uuid,
    email: String,
    ip_address: IpAddr,
    success: bool,
    attempted_at: DateTime<Utc>,
  ) -> Self {
    Self {
      id,
      email,
      ip_address,
      success,
      attempted_at,
    }
  }

  /// Checks if this attempt was made within the given duration from now
  pub fn is_within(&self, duration: Duration) -> bool {
    let cutoff = Utc::now() - duration;
    self.attempted_at >= cutoff
  }

  /// Checks if this attempt was a failure
  pub fn is_failure(&self) -> bool {
    !self.success
  }

  /// Checks if this attempt was successful
  pub fn is_success(&self) -> bool {
    self.success
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use chrono::Duration;

  #[test]
  fn test_user_creation() {
    let user = User::new(
      "test@example.com".to_string(),
      "hashed_password".to_string(),
      "Test User".to_string(),
    );

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.full_name, "Test User");
    assert!(!user.is_email_verified);
    assert!(user.email_verification_token.is_none());
  }

  #[test]
  fn test_user_verify_email() {
    let mut user = User::new(
      "test@example.com".to_string(),
      "hashed_password".to_string(),
      "Test User".to_string(),
    );

    user.set_email_verification_token("token123".to_string(), Duration::hours(24));
    assert!(user.is_email_verification_token_valid("token123"));
    assert!(!user.is_email_verification_token_valid("wrong_token"));

    user.verify_email();
    assert!(user.is_email_verified);
    assert!(user.email_verification_token.is_none());
  }

  #[test]
  fn test_user_password_reset_token() {
    let mut user = User::new(
      "test@example.com".to_string(),
      "hashed_password".to_string(),
      "Test User".to_string(),
    );

    user.set_password_reset_token("reset123".to_string(), Duration::hours(1));
    assert!(user.is_password_reset_token_valid("reset123"));
    assert!(!user.is_password_reset_token_valid("wrong_token"));

    user.clear_password_reset_token();
    assert!(user.password_reset_token.is_none());
  }

  #[test]
  fn test_session_creation() {
    let user_id = Uuid::new_v4();
    let session = Session::with_duration(
      user_id,
      "session_token".to_string(),
      Duration::hours(1),
      Some("127.0.0.1".parse().unwrap()),
      Some("Mozilla/5.0".to_string()),
    );

    assert_eq!(session.user_id, user_id);
    assert!(!session.is_expired());
    assert!(session.is_valid());
  }

  #[test]
  fn test_session_expiration() {
    let user_id = Uuid::new_v4();
    let mut session = Session::new(
      user_id,
      "session_token".to_string(),
      Utc::now() - Duration::seconds(10), // Already expired
      None,
      None,
    );

    assert!(session.is_expired());
    assert!(!session.is_valid());

    // Refresh the session
    session.refresh_with_duration(Duration::hours(1));
    assert!(!session.is_expired());
    assert!(session.is_valid());
  }

  #[test]
  fn test_session_extension() {
    let user_id = Uuid::new_v4();
    let mut session = Session::with_duration(
      user_id,
      "session_token".to_string(),
      Duration::minutes(30),
      None,
      None,
    );

    let original_expiry = session.expires_at;
    session.extend(Duration::minutes(30));

    assert!(session.expires_at > original_expiry);
  }

  #[test]
  fn test_login_attempt_creation() {
    let ip = "192.168.1.1".parse().unwrap();
    let success_attempt = LoginAttempt::success("test@example.com".to_string(), ip);
    let failure_attempt = LoginAttempt::failure("test@example.com".to_string(), ip);

    assert!(success_attempt.is_success());
    assert!(!success_attempt.is_failure());

    assert!(failure_attempt.is_failure());
    assert!(!failure_attempt.is_success());
  }

  #[test]
  fn test_login_attempt_within_duration() {
    let ip = "192.168.1.1".parse().unwrap();
    let attempt = LoginAttempt::new("test@example.com".to_string(), ip, false);

    assert!(attempt.is_within(Duration::minutes(5)));
    assert!(attempt.is_within(Duration::seconds(1)));
  }
}
