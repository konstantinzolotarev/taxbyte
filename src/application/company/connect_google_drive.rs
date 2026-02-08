use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyRepository};
use crate::infrastructure::cloud::{OAuthManager, OAuthTokens};
use crate::infrastructure::security::AesTokenEncryption;

/// Command to initiate OAuth flow
pub struct InitiateOAuthCommand {
  pub company_id: Uuid,
  pub user_id: Uuid,
}

/// Response with OAuth URL
pub struct InitiateOAuthResponse {
  pub authorization_url: String,
  pub state_token: String, // CSRF token
}

/// Command to complete OAuth flow
pub struct CompleteOAuthCommand {
  pub company_id: Uuid,
  pub user_id: Uuid,
  pub code: String,
  pub state: String,
}

/// Use case for connecting Google Drive via OAuth
pub struct ConnectGoogleDriveUseCase {
  oauth_manager: Arc<dyn OAuthManager>,
  company_repo: Arc<dyn CompanyRepository>,
  token_encryption: Arc<AesTokenEncryption>,
}

impl ConnectGoogleDriveUseCase {
  pub fn new(
    oauth_manager: Arc<dyn OAuthManager>,
    company_repo: Arc<dyn CompanyRepository>,
    token_encryption: Arc<AesTokenEncryption>,
  ) -> Self {
    Self {
      oauth_manager,
      company_repo,
      token_encryption,
    }
  }

  /// Initiate OAuth flow - generate authorization URL
  pub async fn initiate_oauth(
    &self,
    cmd: InitiateOAuthCommand,
  ) -> Result<InitiateOAuthResponse, CompanyError> {
    // Verify company exists
    let _company = self
      .company_repo
      .find_by_id(cmd.company_id)
      .await?
      .ok_or(CompanyError::NotFound)?;

    // TODO: Verify user has owner/admin role for this company
    // This requires CompanyMemberRepository which should be injected

    // Generate OAuth URL with company_id as state parameter
    // This allows us to identify which company to update in the callback
    let (auth_url, csrf_token) = self
      .oauth_manager
      .get_authorization_url(Some(cmd.company_id.to_string()));

    // TODO: Store CSRF token in Redis with company_id + user_id (5 min expiry)
    // For now, we use the company_id directly as the state parameter

    Ok(InitiateOAuthResponse {
      authorization_url: auth_url,
      state_token: csrf_token,
    })
  }

  /// Complete OAuth flow - exchange code for tokens
  pub async fn complete_oauth(&self, cmd: CompleteOAuthCommand) -> Result<(), CompanyError> {
    // TODO: Verify CSRF token from Redis

    // Exchange authorization code for tokens
    let tokens = self
      .oauth_manager
      .exchange_code(cmd.code)
      .await
      .map_err(|e| {
        CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
          format!("OAuth exchange failed: {}", e),
        ))
      })?;

    // Encrypt tokens
    let encrypted_access = self.encrypt_token(&tokens.access_token)?;
    let encrypted_refresh = self.encrypt_token(&tokens.refresh_token)?;

    // Calculate expiry
    let expires_at = Utc::now() + chrono::Duration::seconds(tokens.expires_in_seconds as i64);

    // Update company with OAuth tokens
    self
      .company_repo
      .update_oauth_tokens(
        &cmd.company_id,
        encrypted_access,
        encrypted_refresh,
        expires_at,
        cmd.user_id,
      )
      .await?;

    Ok(())
  }

  /// Refresh OAuth access token
  pub async fn refresh_token(&self, company_id: &Uuid) -> Result<OAuthTokens, CompanyError> {
    let company = self
      .company_repo
      .find_by_id(*company_id)
      .await?
      .ok_or(CompanyError::NotFound)?;

    let encrypted_refresh = company.oauth_refresh_token.ok_or_else(|| {
      CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
        "No refresh token found".to_string(),
      ))
    })?;

    // Decrypt refresh token
    let refresh_token = self.decrypt_token(&encrypted_refresh)?;

    // Refresh access token
    let tokens = self
      .oauth_manager
      .refresh_token(refresh_token)
      .await
      .map_err(|e| {
        CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
          format!("Token refresh failed: {}", e),
        ))
      })?;

    // Encrypt new access token
    let encrypted_access = self.encrypt_token(&tokens.access_token)?;
    let encrypted_refresh = self.encrypt_token(&tokens.refresh_token)?;

    // Calculate expiry
    let expires_at = Utc::now() + chrono::Duration::seconds(tokens.expires_in_seconds as i64);

    // Update company with new tokens
    self
      .company_repo
      .update_oauth_tokens(
        company_id,
        encrypted_access,
        encrypted_refresh,
        expires_at,
        company.oauth_connected_by.unwrap_or(Uuid::nil()),
      )
      .await?;

    Ok(tokens)
  }

  fn encrypt_token(&self, token: &str) -> Result<String, CompanyError> {
    self.token_encryption.encrypt(token).map_err(|e| {
      CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
        format!("Token encryption failed: {}", e),
      ))
    })
  }

  fn decrypt_token(&self, encrypted: &str) -> Result<String, CompanyError> {
    self.token_encryption.decrypt(encrypted).map_err(|e| {
      CompanyError::Repository(crate::domain::auth::errors::RepositoryError::QueryFailed(
        format!("Token decryption failed: {}", e),
      ))
    })
  }
}
