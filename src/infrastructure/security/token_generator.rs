use async_trait::async_trait;
use rand::RngCore;

use crate::domain::auth::errors::AuthError;
use crate::domain::auth::ports::TokenGenerator;

/// Secure token generator using cryptographically secure random number generation
pub struct SecureTokenGenerator;

impl SecureTokenGenerator {
  /// Creates a new instance of SecureTokenGenerator
  pub fn new() -> Self {
    Self
  }

  /// Encodes bytes to base64url format (RFC 4648)
  /// Base64url is URL-safe variant that uses - and _ instead of + and /
  /// and omits padding
  fn encode_base64url(bytes: &[u8]) -> String {
    const BASE64URL_CHARS: &[u8; 64] =
      b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut result = String::new();
    let mut i = 0;

    // Process 3 bytes at a time to create 4 base64 characters
    while i + 2 < bytes.len() {
      let b1 = bytes[i];
      let b2 = bytes[i + 1];
      let b3 = bytes[i + 2];

      result.push(BASE64URL_CHARS[(b1 >> 2) as usize] as char);
      result.push(BASE64URL_CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);
      result.push(BASE64URL_CHARS[(((b2 & 0x0f) << 2) | (b3 >> 6)) as usize] as char);
      result.push(BASE64URL_CHARS[(b3 & 0x3f) as usize] as char);

      i += 3;
    }

    // Handle remaining bytes (1 or 2)
    match bytes.len() - i {
      1 => {
        let b1 = bytes[i];
        result.push(BASE64URL_CHARS[(b1 >> 2) as usize] as char);
        result.push(BASE64URL_CHARS[((b1 & 0x03) << 4) as usize] as char);
      }
      2 => {
        let b1 = bytes[i];
        let b2 = bytes[i + 1];
        result.push(BASE64URL_CHARS[(b1 >> 2) as usize] as char);
        result.push(BASE64URL_CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);
        result.push(BASE64URL_CHARS[((b2 & 0x0f) << 2) as usize] as char);
      }
      _ => {}
    }

    result
  }
}

impl Default for SecureTokenGenerator {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl TokenGenerator for SecureTokenGenerator {
  /// Generates a cryptographically secure random token
  ///
  /// Creates a 32-byte random token using the operating system's
  /// cryptographically secure random number generator (OsRng).
  /// The token is then encoded as a base64url string for safe
  /// transmission in URLs and headers.
  ///
  /// # Returns
  /// A base64url-encoded string representing the random token
  ///
  /// # Errors
  /// Returns `AuthError::InternalError` if token generation fails
  async fn generate(&self) -> Result<String, AuthError> {
    let mut rng = rand::rngs::OsRng;
    let mut token_bytes = [0u8; 32];

    // Fill the buffer with cryptographically secure random bytes
    rng.fill_bytes(&mut token_bytes);

    // Encode to base64url format
    let token = Self::encode_base64url(&token_bytes);

    Ok(token)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_generate_creates_unique_tokens() {
    let generator = SecureTokenGenerator::new();

    let token1 = generator.generate().await.unwrap();
    let token2 = generator.generate().await.unwrap();

    // Tokens should be different
    assert_ne!(token1, token2);
  }

  #[tokio::test]
  async fn test_generate_creates_non_empty_token() {
    let generator = SecureTokenGenerator::new();

    let token = generator.generate().await.unwrap();

    // Token should not be empty
    assert!(!token.is_empty());
  }

  #[tokio::test]
  async fn test_generate_creates_base64url_token() {
    let generator = SecureTokenGenerator::new();

    let token = generator.generate().await.unwrap();

    // Base64url should only contain these characters
    assert!(
      token
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
  }

  #[tokio::test]
  async fn test_generate_creates_expected_length_token() {
    let generator = SecureTokenGenerator::new();

    let token = generator.generate().await.unwrap();

    // 32 bytes encoded in base64url without padding should be 43 characters
    // (32 * 8 / 6 = 42.67, rounded up to 43)
    assert_eq!(token.len(), 43);
  }

  #[test]
  fn test_encode_base64url() {
    // Test vector: "hello" -> "aGVsbG8"
    let input = b"hello";
    let expected = "aGVsbG8";
    let result = SecureTokenGenerator::encode_base64url(input);
    assert_eq!(result, expected);
  }

  #[test]
  fn test_encode_base64url_no_padding() {
    // Base64url should not include padding characters
    let input = b"test";
    let result = SecureTokenGenerator::encode_base64url(input);
    assert!(!result.contains('='));
  }

  #[test]
  fn test_encode_base64url_url_safe() {
    // Should use - and _ instead of + and /
    let input = [0xfb, 0xff]; // Would produce +/ in standard base64
    let result = SecureTokenGenerator::encode_base64url(&input);
    assert!(!result.contains('+'));
    assert!(!result.contains('/'));
  }
}
