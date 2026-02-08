use aes_gcm::{
  Aes256Gcm, Nonce,
  aead::{Aead, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};

/// Error types for encryption operations
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
  #[error("Encryption failed: {0}")]
  EncryptionFailed(String),

  #[error("Decryption failed: {0}")]
  DecryptionFailed(String),
}

/// AES-256-GCM encryption for OAuth tokens
///
/// Uses AES-256-GCM (Galois/Counter Mode) which provides both confidentiality
/// and authenticity. Each encryption operation uses a random 96-bit nonce.
pub struct AesTokenEncryption {
  cipher: Aes256Gcm,
}

impl AesTokenEncryption {
  /// Create a new AES token encryption instance
  ///
  /// # Arguments
  /// * `key_base64` - Base64-encoded 32-byte (256-bit) encryption key
  ///
  /// # Example
  /// ```rust
  /// // Generate key with: openssl rand -base64 32
  /// let encryption = AesTokenEncryption::new("your-base64-key-here")?;
  /// ```
  pub fn new(key_base64: &str) -> Result<Self, EncryptionError> {
    let key_bytes = general_purpose::STANDARD
      .decode(key_base64)
      .map_err(|e| EncryptionError::EncryptionFailed(format!("Key decode failed: {}", e)))?;

    if key_bytes.len() != 32 {
      return Err(EncryptionError::EncryptionFailed(
        "Encryption key must be exactly 32 bytes (256 bits)".to_string(),
      ));
    }

    let key_array: &[u8; 32] = key_bytes
      .as_slice()
      .try_into()
      .map_err(|_| EncryptionError::EncryptionFailed("Invalid key length".to_string()))?;

    let cipher = Aes256Gcm::new(key_array.into());

    Ok(Self { cipher })
  }

  /// Encrypt a plaintext token
  ///
  /// Returns a base64-encoded string containing: nonce (12 bytes) + ciphertext
  ///
  /// # Arguments
  /// * `plaintext` - The token to encrypt
  pub fn encrypt(&self, plaintext: &str) -> Result<String, EncryptionError> {
    // Generate random 12-byte (96-bit) nonce
    use rand::RngCore;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt the plaintext
    let ciphertext = self
      .cipher
      .encrypt(nonce, plaintext.as_bytes())
      .map_err(|e| EncryptionError::EncryptionFailed(format!("Encryption failed: {}", e)))?;

    // Combine nonce + ciphertext and base64 encode
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(combined))
  }

  /// Decrypt a ciphertext token
  ///
  /// # Arguments
  /// * `encoded` - Base64-encoded string containing nonce + ciphertext
  pub fn decrypt(&self, encoded: &str) -> Result<String, EncryptionError> {
    // Decode base64
    let combined = general_purpose::STANDARD
      .decode(encoded)
      .map_err(|e| EncryptionError::DecryptionFailed(format!("Base64 decode failed: {}", e)))?;

    if combined.len() < 12 {
      return Err(EncryptionError::DecryptionFailed(
        "Invalid ciphertext format: too short".to_string(),
      ));
    }

    // Split nonce (first 12 bytes) and ciphertext (rest)
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt
    let plaintext_bytes = self
      .cipher
      .decrypt(nonce, ciphertext)
      .map_err(|e| EncryptionError::DecryptionFailed(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext_bytes)
      .map_err(|e| EncryptionError::DecryptionFailed(format!("UTF-8 conversion failed: {}", e)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_encrypt_decrypt_roundtrip() {
    // Generate a test key (32 bytes, base64 encoded)
    let key = general_purpose::STANDARD.encode(&[42u8; 32]);
    let encryption = AesTokenEncryption::new(&key).unwrap();

    let plaintext = "ya29.a0AfB_byABC123...refresh_token";
    let encrypted = encryption.encrypt(plaintext).unwrap();
    let decrypted = encryption.decrypt(&encrypted).unwrap();

    assert_eq!(plaintext, decrypted);
  }

  #[test]
  fn test_encrypt_produces_different_ciphertexts() {
    let key = general_purpose::STANDARD.encode(&[42u8; 32]);
    let encryption = AesTokenEncryption::new(&key).unwrap();

    let plaintext = "same_token";
    let encrypted1 = encryption.encrypt(plaintext).unwrap();
    let encrypted2 = encryption.encrypt(plaintext).unwrap();

    // Different nonces should produce different ciphertexts
    assert_ne!(encrypted1, encrypted2);

    // But both should decrypt to the same plaintext
    assert_eq!(encryption.decrypt(&encrypted1).unwrap(), plaintext);
    assert_eq!(encryption.decrypt(&encrypted2).unwrap(), plaintext);
  }

  #[test]
  fn test_invalid_key_length() {
    let short_key = general_purpose::STANDARD.encode(&[42u8; 16]); // Too short
    assert!(AesTokenEncryption::new(&short_key).is_err());
  }

  #[test]
  fn test_invalid_base64() {
    let key = general_purpose::STANDARD.encode(&[42u8; 32]);
    let encryption = AesTokenEncryption::new(&key).unwrap();

    assert!(encryption.decrypt("not-valid-base64!!!").is_err());
  }

  #[test]
  fn test_tampered_ciphertext() {
    let key = general_purpose::STANDARD.encode(&[42u8; 32]);
    let encryption = AesTokenEncryption::new(&key).unwrap();

    let plaintext = "secret_token";
    let encrypted = encryption.encrypt(plaintext).unwrap();

    // Tamper with the ciphertext
    let mut tampered = general_purpose::STANDARD.decode(&encrypted).unwrap();
    tampered[15] ^= 0xFF; // Flip bits in the middle
    let tampered_encoded = general_purpose::STANDARD.encode(tampered);

    // Decryption should fail due to authentication check
    assert!(encryption.decrypt(&tampered_encoded).is_err());
  }
}
