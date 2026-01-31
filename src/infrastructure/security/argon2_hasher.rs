use argon2::password_hash::SaltString;
use argon2::{
  Algorithm, Argon2, Params, Version,
  password_hash::{
    PasswordHash as Argon2PasswordHash, PasswordHasher as Argon2PasswordHasherTrait,
    PasswordVerifier,
  },
};
use async_trait::async_trait;

use crate::domain::auth::errors::{AuthError, HashError};
use crate::domain::auth::ports::PasswordHasher;
use crate::domain::auth::value_objects::{Password, PasswordHash};

/// Argon2id password hasher implementation
///
/// Uses the Argon2id algorithm with secure parameters:
/// - Memory cost: 19 MiB (19456 KiB)
/// - Time cost: 2 iterations
/// - Parallelism: 1 thread
/// - Algorithm: Argon2id (resistant to both side-channel and GPU attacks)
pub struct Argon2PasswordHasher {
  argon2: Argon2<'static>,
}

impl Argon2PasswordHasher {
  /// Creates a new Argon2PasswordHasher with the specified parameters
  pub fn new() -> Result<Self, AuthError> {
    // Memory cost: 19 MiB = 19456 KiB
    let memory_cost = 19456;
    // Time cost: 2 iterations
    let time_cost = 2;
    // Parallelism: 1 thread
    let parallelism = 1;
    // Output length: 32 bytes (default)
    let output_len = Some(32);

    let params = Params::new(memory_cost, time_cost, parallelism, output_len).map_err(|e| {
      AuthError::Hash(HashError::HashingFailed(format!(
        "Failed to create Argon2 params: {}",
        e
      )))
    })?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    Ok(Self { argon2 })
  }
}

impl Default for Argon2PasswordHasher {
  fn default() -> Self {
    Self::new().expect("Failed to create default Argon2PasswordHasher")
  }
}

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
  /// Hashes a plain text password using Argon2id
  ///
  /// # Arguments
  /// * `password` - The password to hash
  ///
  /// # Returns
  /// * `Ok(PasswordHash)` - The hashed password
  /// * `Err(AuthError)` - If hashing fails
  async fn hash(&self, password: &Password) -> Result<PasswordHash, AuthError> {
    // Generate a random salt using the OS's cryptographically secure RNG
    let salt = SaltString::generate(&mut rand::rngs::OsRng);

    // Hash the password
    let hash = self
      .argon2
      .hash_password(password.as_str().as_bytes(), &salt)
      .map_err(|e| {
        AuthError::Hash(HashError::HashingFailed(format!(
          "Failed to hash password: {}",
          e
        )))
      })?;

    // Convert to our domain PasswordHash type
    PasswordHash::from_hash(hash.to_string()).map_err(|e| {
      AuthError::Hash(HashError::HashingFailed(format!(
        "Invalid hash format: {}",
        e
      )))
    })
  }

  /// Verifies a plain text password against a hashed password
  ///
  /// Uses constant-time comparison to prevent timing attacks
  ///
  /// # Arguments
  /// * `password` - The plain text password to verify
  /// * `hashed_password` - The hashed password to verify against
  ///
  /// # Returns
  /// * `Ok(true)` - If the password matches
  /// * `Ok(false)` - If the password does not match
  /// * `Err(AuthError)` - If verification fails due to invalid hash format
  async fn verify(
    &self,
    password: &Password,
    hashed_password: &PasswordHash,
  ) -> Result<bool, AuthError> {
    // Parse the stored hash
    let parsed_hash = Argon2PasswordHash::new(hashed_password.as_str()).map_err(|e| {
      AuthError::Hash(HashError::VerificationFailed(format!(
        "Invalid hash format: {}",
        e
      )))
    })?;

    // Verify using constant-time comparison (built into argon2's verify_password)
    // The verify_password method uses constant-time comparison internally to prevent timing attacks
    match self
      .argon2
      .verify_password(password.as_str().as_bytes(), &parsed_hash)
    {
      Ok(_) => Ok(true),
      Err(argon2::password_hash::Error::Password) => Ok(false),
      Err(e) => Err(AuthError::Hash(HashError::VerificationFailed(format!(
        "Password verification failed: {}",
        e
      )))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_hash_password() {
    let hasher = Argon2PasswordHasher::new().unwrap();
    let password = Password::new("test_password_123").unwrap();

    let result = hasher.hash(&password).await;
    assert!(result.is_ok());

    let hash = result.unwrap();
    assert!(!hash.as_str().is_empty());
    assert!(hash.as_str().starts_with("$argon2id$"));
  }

  #[tokio::test]
  async fn test_verify_correct_password() {
    let hasher = Argon2PasswordHasher::new().unwrap();
    let password = Password::new("test_password_123").unwrap();

    let hash = hasher.hash(&password).await.unwrap();
    let result = hasher.verify(&password, &hash).await;

    assert!(result.is_ok());
    assert!(result.unwrap());
  }

  #[tokio::test]
  async fn test_verify_incorrect_password() {
    let hasher = Argon2PasswordHasher::new().unwrap();
    let password = Password::new("test_password_123").unwrap();
    let wrong_password = Password::new("wrong_password").unwrap();

    let hash = hasher.hash(&password).await.unwrap();
    let result = hasher.verify(&wrong_password, &hash).await;

    assert!(result.is_ok());
    assert!(!result.unwrap());
  }

  #[tokio::test]
  async fn test_hash_produces_different_salts() {
    let hasher = Argon2PasswordHasher::new().unwrap();
    let password = Password::new("test_password_123").unwrap();

    let hash1 = hasher.hash(&password).await.unwrap();
    let hash2 = hasher.hash(&password).await.unwrap();

    // Same password should produce different hashes due to random salt
    assert_ne!(hash1.as_str(), hash2.as_str());

    // Both should verify correctly
    assert!(hasher.verify(&password, &hash1).await.unwrap());
    assert!(hasher.verify(&password, &hash2).await.unwrap());
  }

  #[tokio::test]
  async fn test_verify_invalid_hash_format() {
    // Create an invalid hash (not a proper Argon2 hash)
    let result = PasswordHash::from_hash("invalid_hash");
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_argon2_parameters() {
    let hasher = Argon2PasswordHasher::new().unwrap();
    let password = Password::new("test_password_123").unwrap();

    let hash = hasher.hash(&password).await.unwrap();
    let hash_str = hash.as_str();

    // Verify it's using Argon2id
    assert!(hash_str.starts_with("$argon2id$"));

    // Parse the hash to check parameters
    let parsed = Argon2PasswordHash::new(hash_str).unwrap();

    // Verify parameters are set correctly
    assert_eq!(parsed.version, Some(Version::V0x13 as u32));
  }
}
