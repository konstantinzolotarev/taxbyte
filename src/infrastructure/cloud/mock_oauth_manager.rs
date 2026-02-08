use super::oauth_manager::{OAuthManager, OAuthTokens};
/// Mock OAuth manager for development/testing
///
/// This mock implementation simulates the OAuth flow without actually
/// contacting Google's OAuth servers. Useful for:
/// - Local development without OAuth credentials
/// - Automated testing
/// - CI/CD environments
///
/// To enable mock OAuth, set MOCK_OAUTH=true in your environment
use async_trait::async_trait;

pub struct MockOAuthManager {
  redirect_url: String,
}

impl MockOAuthManager {
  pub fn new(
    _client_id: String,
    _client_secret: String,
    redirect_url: String,
  ) -> Result<Self, String> {
    Ok(Self { redirect_url })
  }

  /// Generate a mock authorization URL
  ///
  /// Returns a URL to the local mock OAuth page instead of Google's consent screen
  pub fn get_authorization_url(&self, state: Option<String>) -> (String, String) {
    // Use provided state (e.g., company_id) or generate a random one
    let state = state.unwrap_or_else(|| format!("mock-state-{}", uuid::Uuid::new_v4()));
    // Simple URL encoding for the redirect_uri parameter
    let encoded_redirect = self.redirect_url.replace(':', "%3A").replace('/', "%2F");
    let auth_url = format!(
      "/dev/mock-oauth?state={}&redirect_uri={}",
      state, encoded_redirect
    );
    (auth_url, state)
  }

  /// Mock token exchange - returns fake tokens
  pub async fn exchange_code(&self, _code: String) -> Result<OAuthTokens, String> {
    // Simulate a small delay like a real API call
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(OAuthTokens {
      access_token: format!("mock-access-token-{}", uuid::Uuid::new_v4()),
      refresh_token: format!("mock-refresh-token-{}", uuid::Uuid::new_v4()),
      expires_in_seconds: 3600, // 1 hour
    })
  }

  /// Mock token refresh - returns new fake tokens
  pub async fn refresh_token(&self, _refresh_token: String) -> Result<OAuthTokens, String> {
    // Simulate a small delay like a real API call
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(OAuthTokens {
      access_token: format!("mock-refreshed-access-token-{}", uuid::Uuid::new_v4()),
      refresh_token: _refresh_token, // Keep the same refresh token
      expires_in_seconds: 3600,
    })
  }
}

// Implement OAuthManager trait for MockOAuthManager
#[async_trait]
impl OAuthManager for MockOAuthManager {
  fn get_authorization_url(&self, state: Option<String>) -> (String, String) {
    MockOAuthManager::get_authorization_url(self, state)
  }

  async fn exchange_code(&self, code: String) -> Result<OAuthTokens, String> {
    self.exchange_code(code).await
  }

  async fn refresh_token(&self, refresh_token: String) -> Result<OAuthTokens, String> {
    self.refresh_token(refresh_token).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_mock_oauth_manager_creation() {
    let manager = MockOAuthManager::new(
      "mock-client-id".to_string(),
      "mock-client-secret".to_string(),
      "http://localhost:8080/oauth/callback".to_string(),
    );

    assert!(manager.is_ok());
  }

  #[test]
  fn test_get_authorization_url() {
    let manager = MockOAuthManager::new(
      "mock-client-id".to_string(),
      "mock-client-secret".to_string(),
      "http://localhost:8080/oauth/callback".to_string(),
    )
    .unwrap();

    let (auth_url, state) = manager.get_authorization_url(None);

    assert!(auth_url.contains("/dev/mock-oauth"));
    assert!(auth_url.contains(&state));
    assert!(!state.is_empty());
  }

  #[tokio::test]
  async fn test_exchange_code() {
    let manager = MockOAuthManager::new(
      "mock-client-id".to_string(),
      "mock-client-secret".to_string(),
      "http://localhost:8080/oauth/callback".to_string(),
    )
    .unwrap();

    let tokens = manager.exchange_code("mock-code".to_string()).await;

    assert!(tokens.is_ok());
    let tokens = tokens.unwrap();
    assert!(tokens.access_token.starts_with("mock-access-token-"));
    assert!(tokens.refresh_token.starts_with("mock-refresh-token-"));
    assert_eq!(tokens.expires_in_seconds, 3600);
  }

  #[tokio::test]
  async fn test_refresh_token() {
    let manager = MockOAuthManager::new(
      "mock-client-id".to_string(),
      "mock-client-secret".to_string(),
      "http://localhost:8080/oauth/callback".to_string(),
    )
    .unwrap();

    let old_refresh_token = "mock-refresh-token-123".to_string();
    let tokens = manager.refresh_token(old_refresh_token.clone()).await;

    assert!(tokens.is_ok());
    let tokens = tokens.unwrap();
    assert!(
      tokens
        .access_token
        .starts_with("mock-refreshed-access-token-")
    );
    assert_eq!(tokens.refresh_token, old_refresh_token); // Should keep same refresh token
  }
}
