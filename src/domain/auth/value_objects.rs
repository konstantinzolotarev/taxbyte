use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash as Argon2PasswordHash, PasswordHasher, PasswordVerifier};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;
use validator::ValidateEmail;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error)]
pub enum ValueObjectError {
  #[error("Invalid email format: {0}")]
  InvalidEmail(String),

  #[error("Password is too short (minimum 8 characters)")]
  PasswordTooShort,

  #[error("Password is too long (maximum 128 characters)")]
  PasswordTooLong,

  #[error("Invalid password hash format")]
  InvalidPasswordHash,

  #[error("Password hashing failed: {0}")]
  HashingFailed(String),

  #[error("Password verification failed: {0}")]
  VerificationFailed(String),

  #[error("Invalid token format")]
  InvalidToken,

  #[error("Token generation failed: {0}")]
  TokenGenerationFailed(String),
}

// ============================================================================
// Email Value Object
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
  /// Creates a new Email after validation
  pub fn new(email: impl Into<String>) -> Result<Self, ValueObjectError> {
    let email = email.into();

    if !email.validate_email() {
      return Err(ValueObjectError::InvalidEmail(email));
    }

    // Normalize to lowercase
    Ok(Self(email.to_lowercase()))
  }

  /// Returns the email as a string slice
  pub fn as_str(&self) -> &str {
    &self.0
  }

  /// Consumes self and returns the inner String
  pub fn into_inner(self) -> String {
    self.0
  }
}

impl fmt::Display for Email {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl AsRef<str> for Email {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

// ============================================================================
// Password Value Object (Plain Password - Never Stored)
// ============================================================================

#[derive(Clone)]
pub struct Password(String);

impl Password {
  const MIN_LENGTH: usize = 8;
  const MAX_LENGTH: usize = 128;

  /// Creates a new Password after validation
  pub fn new(password: impl Into<String>) -> Result<Self, ValueObjectError> {
    let password = password.into();

    if password.len() < Self::MIN_LENGTH {
      return Err(ValueObjectError::PasswordTooShort);
    }

    if password.len() > Self::MAX_LENGTH {
      return Err(ValueObjectError::PasswordTooLong);
    }

    Ok(Self(password))
  }

  /// Hashes the password using Argon2id
  pub fn hash(&self) -> Result<PasswordHash, ValueObjectError> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
      .hash_password(self.0.as_bytes(), &salt)
      .map_err(|e| ValueObjectError::HashingFailed(e.to_string()))?;

    Ok(PasswordHash(hash.to_string()))
  }

  /// Returns the password as a string slice (use with caution)
  pub fn as_str(&self) -> &str {
    &self.0
  }
}

// Implement Debug without exposing the password
impl fmt::Debug for Password {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("Password(***)")
  }
}

// Implement Display without exposing the password
impl fmt::Display for Password {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("***")
  }
}

// Ensure Password is securely dropped
impl Drop for Password {
  fn drop(&mut self) {
    // Zero out the password memory
    use std::ptr;
    unsafe {
      ptr::write_volatile(self.0.as_mut_ptr(), 0u8.wrapping_mul(self.0.len() as u8));
    }
  }
}

// ============================================================================
// PasswordHash Value Object (Argon2id Hash)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHash(String);

impl PasswordHash {
  /// Creates a new PasswordHash from an existing hash string
  pub fn from_hash(hash: impl Into<String>) -> Result<Self, ValueObjectError> {
    let hash = hash.into();

    // Validate it's a proper Argon2 hash
    Argon2PasswordHash::new(&hash).map_err(|_| ValueObjectError::InvalidPasswordHash)?;

    Ok(Self(hash))
  }

  /// Verifies a password against this hash
  pub fn verify(&self, password: &Password) -> Result<bool, ValueObjectError> {
    let parsed_hash = Argon2PasswordHash::new(&self.0)
      .map_err(|e| ValueObjectError::VerificationFailed(e.to_string()))?;

    let argon2 = Argon2::default();

    Ok(
      argon2
        .verify_password(password.as_str().as_bytes(), &parsed_hash)
        .is_ok(),
    )
  }

  /// Returns the hash as a string slice
  pub fn as_str(&self) -> &str {
    &self.0
  }

  /// Consumes self and returns the inner String
  pub fn into_inner(self) -> String {
    self.0
  }
}

impl fmt::Display for PasswordHash {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

// ============================================================================
// SessionToken Value Object (Random Secure Token)
// ============================================================================

#[derive(Clone)]
pub struct SessionToken(String);

impl SessionToken {
  const TOKEN_LENGTH: usize = 32; // 32 bytes = 256 bits

  /// Generates a new random session token
  pub fn generate() -> Result<Self, ValueObjectError> {
    use rand::Rng;

    let token: [u8; Self::TOKEN_LENGTH] = rand::rngs::OsRng.sample(rand::distributions::Standard);

    let token_string = hex::encode(token);
    Ok(Self(token_string))
  }

  /// Creates a SessionToken from an existing token string
  pub fn from_string(token: impl Into<String>) -> Result<Self, ValueObjectError> {
    let token = token.into();

    // Validate token is hex and correct length
    if token.len() != Self::TOKEN_LENGTH * 2 {
      return Err(ValueObjectError::InvalidToken);
    }

    if !token.chars().all(|c| c.is_ascii_hexdigit()) {
      return Err(ValueObjectError::InvalidToken);
    }

    Ok(Self(token))
  }

  /// Creates a hash of this token for storage
  pub fn hash(&self) -> TokenHash {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(self.0.as_bytes());
    let result = hasher.finalize();

    TokenHash(hex::encode(result))
  }

  /// Returns the token as a string slice (use with caution)
  pub fn as_str(&self) -> &str {
    &self.0
  }

  /// Consumes self and returns the inner String
  pub fn into_inner(self) -> String {
    self.0
  }
}

// Implement Debug without exposing the token
impl fmt::Debug for SessionToken {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("SessionToken(***)")
  }
}

// Implement Display without exposing the token
impl fmt::Display for SessionToken {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("***")
  }
}

// ============================================================================
// TokenHash Value Object (SHA-256 Hash of Token)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenHash(String);

impl TokenHash {
  /// Creates a TokenHash from an existing hash string
  pub fn from_hash(hash: impl Into<String>) -> Result<Self, ValueObjectError> {
    let hash = hash.into();

    // SHA-256 produces 64 hex characters
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
      return Err(ValueObjectError::InvalidToken);
    }

    Ok(Self(hash))
  }

  /// Verifies a token against this hash
  pub fn verify(&self, token: &SessionToken) -> bool {
    let token_hash = token.hash();
    self.0 == token_hash.0
  }

  /// Returns the hash as a string slice
  pub fn as_str(&self) -> &str {
    &self.0
  }

  /// Consumes self and returns the inner String
  pub fn into_inner(self) -> String {
    self.0
  }
}

impl fmt::Display for TokenHash {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

// ============================================================================
// UserId Value Object
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

impl UserId {
  /// Creates a new random UserId
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Creates a UserId from an existing UUID
  pub fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }

  /// Returns the inner UUID
  pub fn into_inner(self) -> Uuid {
    self.0
  }

  /// Returns a reference to the inner UUID
  pub fn as_uuid(&self) -> &Uuid {
    &self.0
  }
}

impl Default for UserId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for UserId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl From<Uuid> for UserId {
  fn from(uuid: Uuid) -> Self {
    Self(uuid)
  }
}

impl From<UserId> for Uuid {
  fn from(user_id: UserId) -> Self {
    user_id.0
  }
}

// ============================================================================
// SessionId Value Object
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(Uuid);

impl SessionId {
  /// Creates a new random SessionId
  pub fn new() -> Self {
    Self(Uuid::new_v4())
  }

  /// Creates a SessionId from an existing UUID
  pub fn from_uuid(uuid: Uuid) -> Self {
    Self(uuid)
  }

  /// Returns the inner UUID
  pub fn into_inner(self) -> Uuid {
    self.0
  }

  /// Returns a reference to the inner UUID
  pub fn as_uuid(&self) -> &Uuid {
    &self.0
  }
}

impl Default for SessionId {
  fn default() -> Self {
    Self::new()
  }
}

impl fmt::Display for SessionId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl From<Uuid> for SessionId {
  fn from(uuid: Uuid) -> Self {
    Self(uuid)
  }
}

impl From<SessionId> for Uuid {
  fn from(session_id: SessionId) -> Self {
    session_id.0
  }
}

// ============================================================================
// FailureReason Enum
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureReason {
  /// Invalid email or password
  InvalidCredentials,

  /// Account is locked due to too many failed attempts
  AccountLocked,

  /// Account has been disabled
  AccountDisabled,

  /// Email not verified
  EmailNotVerified,

  /// Session expired
  SessionExpired,

  /// Invalid session token
  InvalidToken,

  /// Session not found
  SessionNotFound,

  /// User not found
  UserNotFound,

  /// Rate limit exceeded
  RateLimitExceeded,

  /// Internal server error
  InternalError,
}

impl fmt::Display for FailureReason {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::InvalidCredentials => write!(f, "Invalid credentials"),
      Self::AccountLocked => write!(f, "Account is locked"),
      Self::AccountDisabled => write!(f, "Account is disabled"),
      Self::EmailNotVerified => write!(f, "Email not verified"),
      Self::SessionExpired => write!(f, "Session expired"),
      Self::InvalidToken => write!(f, "Invalid token"),
      Self::SessionNotFound => write!(f, "Session not found"),
      Self::UserNotFound => write!(f, "User not found"),
      Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
      Self::InternalError => write!(f, "Internal error"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_email_validation() {
    // Valid emails
    assert!(Email::new("test@example.com").is_ok());
    assert!(Email::new("user.name@domain.co.uk").is_ok());

    // Invalid emails
    assert!(Email::new("invalid").is_err());
    assert!(Email::new("@example.com").is_err());
    assert!(Email::new("test@").is_err());
  }

  #[test]
  fn test_email_normalization() {
    let email = Email::new("Test@Example.COM").unwrap();
    assert_eq!(email.as_str(), "test@example.com");
  }

  #[test]
  fn test_password_validation() {
    // Valid password
    assert!(Password::new("password123").is_ok());

    // Too short
    assert!(matches!(
      Password::new("short"),
      Err(ValueObjectError::PasswordTooShort)
    ));

    // Too long
    let long_password = "a".repeat(129);
    assert!(matches!(
      Password::new(long_password),
      Err(ValueObjectError::PasswordTooLong)
    ));
  }

  #[test]
  fn test_password_hashing_and_verification() {
    let password = Password::new("mysecretpassword").unwrap();
    let hash = password.hash().unwrap();

    // Should verify correctly
    assert!(hash.verify(&password).unwrap());

    // Should not verify with wrong password
    let wrong_password = Password::new("wrongpassword").unwrap();
    assert!(!hash.verify(&wrong_password).unwrap());
  }

  #[test]
  fn test_session_token_generation() {
    let token1 = SessionToken::generate().unwrap();
    let token2 = SessionToken::generate().unwrap();

    // Tokens should be different
    assert_ne!(token1.as_str(), token2.as_str());

    // Token should be correct length (64 hex characters for 32 bytes)
    assert_eq!(token1.as_str().len(), 64);
  }

  #[test]
  fn test_token_hashing_and_verification() {
    let token = SessionToken::generate().unwrap();
    let hash = token.hash();

    // Should verify correctly
    assert!(hash.verify(&token));

    // Should not verify with different token
    let other_token = SessionToken::generate().unwrap();
    assert!(!hash.verify(&other_token));
  }

  #[test]
  fn test_user_id_creation() {
    let user_id = UserId::new();
    let uuid = user_id.into_inner();

    let user_id2 = UserId::from_uuid(uuid);
    assert_eq!(user_id2.into_inner(), uuid);
  }

  #[test]
  fn test_session_id_creation() {
    let session_id = SessionId::new();
    let uuid = session_id.into_inner();

    let session_id2 = SessionId::from_uuid(uuid);
    assert_eq!(session_id2.into_inner(), uuid);
  }

  #[test]
  fn test_failure_reason_display() {
    assert_eq!(
      FailureReason::InvalidCredentials.to_string(),
      "Invalid credentials"
    );
    assert_eq!(
      FailureReason::AccountLocked.to_string(),
      "Account is locked"
    );
  }
}
