use super::{GoogleDriveAdapter, GoogleDriveOAuthAdapter, NoOpCloudStorage};
use crate::application::company::ConnectGoogleDriveUseCase;
use crate::domain::company::{Company, StorageConfig, StorageProvider};
use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;
use crate::infrastructure::security::AesTokenEncryption;
use std::sync::Arc;

pub struct CloudStorageFactory;

impl CloudStorageFactory {
  /// Create a cloud storage adapter (legacy method without OAuth support)
  ///
  /// This method maintains backward compatibility but does NOT support OAuth.
  /// Use `create_with_oauth` for full OAuth support.
  pub async fn create(
    provider: Option<&String>,
    _config_json: Option<&String>,
  ) -> Arc<dyn CloudStorage> {
    // Parse provider type
    let provider_type = provider
      .and_then(|p| p.parse::<StorageProvider>().ok())
      .unwrap_or_default();

    match provider_type {
      StorageProvider::None => {
        tracing::debug!("Using NoOpCloudStorage for company");
        Arc::new(NoOpCloudStorage::new())
      }

      StorageProvider::GoogleDrive => {
        // OAuth is configured separately in the database (oauth_refresh_token field)
        // This method doesn't have access to the Company entity, so it can't use OAuth
        // The config JSON should NOT contain service_account_key anymore

        tracing::warn!(
          "Google Drive selected but OAuth tokens must be accessed via create_with_oauth(). \
           This legacy method doesn't support OAuth. Using NoOpCloudStorage."
        );

        Arc::new(NoOpCloudStorage::new())
      }
    }
  }

  /// Create a cloud storage adapter with full OAuth support
  ///
  /// # Arguments
  /// * `provider` - Storage provider type (google_drive, s3, etc.)
  /// * `config_json` - JSON configuration for the storage provider
  /// * `company` - Company entity with OAuth token fields
  /// * `token_encryption` - Encryption service for decrypting OAuth tokens
  /// * `connect_use_case` - Optional use case for refreshing tokens
  /// * `oauth_client_id` - OAuth client ID from Google Cloud Console (for token refresh)
  /// * `oauth_client_secret` - OAuth client secret from Google Cloud Console (for token refresh)
  pub async fn create_with_oauth(
    provider: Option<&String>,
    config_json: Option<&String>,
    company: &Company,
    token_encryption: &AesTokenEncryption,
    connect_use_case: Option<&ConnectGoogleDriveUseCase>,
    oauth_client_id: Option<&str>,
    oauth_client_secret: Option<&str>,
  ) -> Arc<dyn CloudStorage> {
    // Parse provider type
    let provider_type = provider
      .and_then(|p| p.parse::<StorageProvider>().ok())
      .unwrap_or_default();

    match provider_type {
      StorageProvider::None => {
        tracing::debug!("Using NoOpCloudStorage for company");
        Arc::new(NoOpCloudStorage::new())
      }

      StorageProvider::GoogleDrive => {
        // Try OAuth first (preferred)
        if let Some(oauth_adapter) = Self::try_create_oauth_adapter(
          company,
          config_json,
          token_encryption,
          connect_use_case,
          oauth_client_id,
          oauth_client_secret,
        )
        .await
        {
          return Arc::new(oauth_adapter);
        }

        // Fall back to service account (deprecated)
        if let Some(config_str) = config_json {
          match serde_json::from_str::<StorageConfig>(config_str) {
            Ok(StorageConfig::GoogleDrive(config)) => {
              match Self::create_google_drive_adapter(config).await {
                Ok(adapter) => {
                  tracing::warn!(
                    "Using deprecated service account authentication for Google Drive. Please migrate to OAuth."
                  );
                  return Arc::new(adapter);
                }
                Err(e) => {
                  tracing::warn!(
                    "Failed to create Google Drive adapter: {}. Using NoOpCloudStorage.",
                    e
                  );
                }
              }
            }
            Ok(_) => {
              tracing::warn!(
                "Google Drive provider selected but config doesn't match. Using NoOpCloudStorage."
              );
            }
            Err(e) => {
              tracing::warn!(
                "Failed to parse Google Drive config: {}. Using NoOpCloudStorage.",
                e
              );
            }
          }
        } else {
          tracing::warn!(
            "Google Drive provider selected but no configuration found. Using NoOpCloudStorage."
          );
        }

        Arc::new(NoOpCloudStorage::new())
      }
    }
  }

  /// Try to create OAuth adapter if company has OAuth tokens
  async fn try_create_oauth_adapter(
    company: &Company,
    config_json: Option<&String>,
    token_encryption: &AesTokenEncryption,
    connect_use_case: Option<&ConnectGoogleDriveUseCase>,
    oauth_client_id: Option<&str>,
    oauth_client_secret: Option<&str>,
  ) -> Option<GoogleDriveOAuthAdapter> {
    // Check if company has OAuth connection
    if !company.has_oauth_connection() {
      return None;
    }

    // Check if token needs refresh
    if company.needs_token_refresh() {
      if let Some(use_case) = connect_use_case {
        tracing::info!(
          "OAuth token expired for company {}. Attempting refresh...",
          company.id
        );

        match use_case.refresh_token(&company.id).await {
          Ok(tokens) => {
            tracing::info!("OAuth token refreshed successfully");
            // Use the refreshed token
            return Self::create_oauth_adapter_from_token(
              oauth_client_id?,
              oauth_client_secret?,
              &tokens.refresh_token,
              config_json,
            )
            .await
            .ok();
          }
          Err(e) => {
            tracing::error!(
              "Failed to refresh OAuth token for company {}: {}",
              company.id,
              e
            );
            return None;
          }
        }
      } else {
        tracing::warn!("OAuth token expired but no refresh use case provided. Uploads will fail.");
        return None;
      }
    }

    // Decrypt refresh token
    let encrypted_refresh = company.oauth_refresh_token.as_ref()?;
    let refresh_token = match token_encryption.decrypt(encrypted_refresh) {
      Ok(token) => token,
      Err(e) => {
        tracing::error!("Failed to decrypt OAuth refresh token: {}", e);
        return None;
      }
    };

    // Create OAuth adapter
    Self::create_oauth_adapter_from_token(
      oauth_client_id?,
      oauth_client_secret?,
      &refresh_token,
      config_json,
    )
    .await
    .ok()
  }

  /// Create OAuth adapter from refresh token
  async fn create_oauth_adapter_from_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    config_json: Option<&String>,
  ) -> Result<GoogleDriveOAuthAdapter, InvoiceError> {
    // Parse config to get parent folder ID
    let parent_folder_id = if let Some(config_str) = config_json {
      match serde_json::from_str::<StorageConfig>(config_str) {
        Ok(StorageConfig::GoogleDrive(config)) => config.parent_folder_id,
        _ => None,
      }
    } else {
      None
    };

    GoogleDriveOAuthAdapter::new(client_id, client_secret, refresh_token, parent_folder_id).await
  }

  /// Create service account adapter (deprecated)
  async fn create_google_drive_adapter(
    config: crate::domain::company::GoogleDriveConfig,
  ) -> Result<GoogleDriveAdapter, InvoiceError> {
    // Service account key is now optional (OAuth is preferred)
    let service_account_key = config.service_account_key.ok_or_else(|| {
      InvoiceError::CloudStorageAuthFailed(
        "Service account key is required for legacy authentication. Please use OAuth instead."
          .to_string(),
      )
    })?;

    // Decode service account key if it's base64
    let key_content = if service_account_key.starts_with('{') {
      // Already JSON
      service_account_key.clone()
    } else {
      // Assume it's a file path
      std::fs::read_to_string(&service_account_key).map_err(|e| {
        InvoiceError::CloudStorageAuthFailed(format!("Failed to read key file: {}", e))
      })?
    };

    GoogleDriveAdapter::new_from_json(&key_content, config.parent_folder_id).await
  }
}
