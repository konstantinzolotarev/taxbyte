use async_trait::async_trait;
use oauth2::reqwest::async_http_client;
use oauth2::{
  AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, RefreshToken, Scope,
  TokenResponse, TokenUrl, basic::BasicClient,
};

/// OAuth tokens returned from Google
#[derive(Debug, Clone)]
pub struct OAuthTokens {
  pub access_token: String,
  pub refresh_token: String,
  pub expires_in_seconds: u64,
}

/// Trait for OAuth managers (allows both real and mock implementations)
#[async_trait]
pub trait OAuthManager: Send + Sync {
  /// Generate authorization URL for OAuth consent screen
  /// Returns (auth_url, state_token)
  /// The state parameter is used for CSRF protection and can encode additional data
  fn get_authorization_url(&self, state: Option<String>) -> (String, String);

  /// Exchange authorization code for tokens
  async fn exchange_code(&self, code: String) -> Result<OAuthTokens, String>;

  /// Refresh access token using refresh token
  async fn refresh_token(&self, refresh_token: String) -> Result<OAuthTokens, String>;
}

/// Google OAuth 2.0 manager for Drive API access
pub struct GoogleOAuthManager {
  client: BasicClient,
}

impl GoogleOAuthManager {
  /// Create a new OAuth manager
  ///
  /// # Arguments
  /// * `client_id` - OAuth client ID from Google Cloud Console
  /// * `client_secret` - OAuth client secret from Google Cloud Console
  /// * `redirect_url` - Redirect URL (must match Google Cloud Console settings)
  pub fn new(
    client_id: String,
    client_secret: String,
    redirect_url: String,
  ) -> Result<Self, String> {
    let client = BasicClient::new(
      ClientId::new(client_id),
      Some(ClientSecret::new(client_secret)),
      AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
        .map_err(|e| format!("Invalid auth URL: {}", e))?,
      Some(
        TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
          .map_err(|e| format!("Invalid token URL: {}", e))?,
      ),
    )
    .set_redirect_uri(
      RedirectUrl::new(redirect_url).map_err(|e| format!("Invalid redirect URL: {}", e))?,
    );

    Ok(Self { client })
  }

  /// Generate authorization URL for user consent
  ///
  /// Returns (authorization_url, csrf_state_token)
  pub fn get_authorization_url(&self, state: Option<String>) -> (String, String) {
    // Use provided state or generate a random one
    let csrf_token = state.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let csrf_token_obj = CsrfToken::new(csrf_token.clone());

    let (auth_url, _) = self
      .client
      .authorize_url(|| csrf_token_obj.clone())
      // Request drive.file scope (access to app-created files only)
      .add_scope(Scope::new(
        "https://www.googleapis.com/auth/drive.file".to_string(),
      ))
      // Request offline access (to get refresh token)
      .add_extra_param("access_type", "offline")
      // Force consent screen to ensure we get a refresh token
      .add_extra_param("prompt", "consent")
      // Note: PKCE removed - not needed for confidential clients (server-side with client secret)
      .url();

    (auth_url.to_string(), csrf_token)
  }

  /// Exchange authorization code for tokens
  ///
  /// # Arguments
  /// * `code` - Authorization code from OAuth callback
  pub async fn exchange_code(&self, code: String) -> Result<OAuthTokens, String> {
    let token_response = self
      .client
      .exchange_code(AuthorizationCode::new(code))
      .request_async(async_http_client)
      .await
      .map_err(|e| format!("Token exchange failed: {}", e))?;

    let refresh_token = token_response
      .refresh_token()
      .ok_or("No refresh token received from Google")?;

    Ok(OAuthTokens {
      access_token: token_response.access_token().secret().clone(),
      refresh_token: refresh_token.secret().clone(),
      expires_in_seconds: token_response
        .expires_in()
        .map(|d| d.as_secs())
        .unwrap_or(3600),
    })
  }

  /// Refresh access token using refresh token
  ///
  /// # Arguments
  /// * `refresh_token` - Refresh token from previous OAuth flow
  pub async fn refresh_token(&self, refresh_token: String) -> Result<OAuthTokens, String> {
    let token_response = self
      .client
      .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
      .request_async(async_http_client)
      .await
      .map_err(|e| format!("Token refresh failed: {}", e))?;

    Ok(OAuthTokens {
      access_token: token_response.access_token().secret().clone(),
      // Keep the existing refresh token (Google doesn't always return a new one)
      refresh_token,
      expires_in_seconds: token_response
        .expires_in()
        .map(|d| d.as_secs())
        .unwrap_or(3600),
    })
  }
}

// Implement OAuthManager trait for GoogleOAuthManager
#[async_trait]
impl OAuthManager for GoogleOAuthManager {
  fn get_authorization_url(&self, state: Option<String>) -> (String, String) {
    GoogleOAuthManager::get_authorization_url(self, state)
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
  fn test_oauth_manager_creation() {
    let manager = GoogleOAuthManager::new(
      "test-client-id".to_string(),
      "test-client-secret".to_string(),
      "http://localhost:8080/oauth/callback".to_string(),
    );

    assert!(manager.is_ok());
  }

  #[test]
  fn test_get_authorization_url() {
    let manager = GoogleOAuthManager::new(
      "test-client-id".to_string(),
      "test-client-secret".to_string(),
      "http://localhost:8080/oauth/callback".to_string(),
    )
    .unwrap();

    let (auth_url, csrf_token) = manager.get_authorization_url();

    assert!(auth_url.contains("accounts.google.com"));
    assert!(auth_url.contains("drive.file"));
    assert!(auth_url.contains("access_type=offline"));
    assert!(auth_url.contains("prompt=consent"));
    assert!(!csrf_token.is_empty());
  }
}
