use super::{GoogleDriveAdapter, NoOpCloudStorage, S3Adapter};
use crate::domain::company::{StorageConfig, StorageProvider};
use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;
use std::sync::Arc;

pub struct CloudStorageFactory;

impl CloudStorageFactory {
  /// Create a cloud storage adapter based on company configuration
  pub async fn create(
    provider: Option<&String>,
    config_json: Option<&String>,
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
        if let Some(config_str) = config_json {
          match serde_json::from_str::<StorageConfig>(config_str) {
            Ok(StorageConfig::GoogleDrive(config)) => {
              match Self::create_google_drive_adapter(config).await {
                Ok(adapter) => {
                  tracing::info!("Google Drive adapter created for company");
                  Arc::new(adapter)
                }
                Err(e) => {
                  tracing::warn!(
                    "Failed to create Google Drive adapter: {}. Using NoOpCloudStorage.",
                    e
                  );
                  Arc::new(NoOpCloudStorage::new())
                }
              }
            }
            Ok(_) => {
              tracing::warn!(
                "Google Drive provider selected but config doesn't match. Using NoOpCloudStorage."
              );
              Arc::new(NoOpCloudStorage::new())
            }
            Err(e) => {
              tracing::warn!(
                "Failed to parse Google Drive config: {}. Using NoOpCloudStorage.",
                e
              );
              Arc::new(NoOpCloudStorage::new())
            }
          }
        } else {
          tracing::warn!(
            "Google Drive provider selected but no configuration found. Using NoOpCloudStorage."
          );
          Arc::new(NoOpCloudStorage::new())
        }
      }

      StorageProvider::S3 => {
        if let Some(config_str) = config_json {
          match serde_json::from_str::<StorageConfig>(config_str) {
            Ok(StorageConfig::S3(config)) => match Self::create_s3_adapter(config).await {
              Ok(adapter) => {
                tracing::info!("S3 adapter created for company");
                Arc::new(adapter)
              }
              Err(e) => {
                tracing::warn!(
                  "Failed to create S3 adapter: {}. Using NoOpCloudStorage.",
                  e
                );
                Arc::new(NoOpCloudStorage::new())
              }
            },
            Ok(_) => {
              tracing::warn!(
                "S3 provider selected but config doesn't match. Using NoOpCloudStorage."
              );
              Arc::new(NoOpCloudStorage::new())
            }
            Err(e) => {
              tracing::warn!("Failed to parse S3 config: {}. Using NoOpCloudStorage.", e);
              Arc::new(NoOpCloudStorage::new())
            }
          }
        } else {
          tracing::warn!(
            "S3 provider selected but no configuration found. Using NoOpCloudStorage."
          );
          Arc::new(NoOpCloudStorage::new())
        }
      }
    }
  }

  async fn create_google_drive_adapter(
    config: crate::domain::company::GoogleDriveConfig,
  ) -> Result<GoogleDriveAdapter, InvoiceError> {
    // Decode service account key if it's base64
    let key_content = if config.service_account_key.starts_with('{') {
      // Already JSON
      config.service_account_key.clone()
    } else {
      // Assume it's a file path
      std::fs::read_to_string(&config.service_account_key).map_err(|e| {
        InvoiceError::CloudStorageAuthFailed(format!("Failed to read key file: {}", e))
      })?
    };

    GoogleDriveAdapter::new_from_json(&key_content, config.parent_folder_id).await
  }

  async fn create_s3_adapter(
    config: crate::domain::company::S3Config,
  ) -> Result<S3Adapter, InvoiceError> {
    S3Adapter::new(
      &config.bucket,
      &config.region,
      &config.access_key_id,
      &config.secret_access_key,
      config.prefix,
    )
    .await
  }
}
