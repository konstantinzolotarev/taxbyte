use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Storage provider type for cloud uploads
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageProvider {
  #[default]
  None,
  GoogleDrive,
}

impl StorageProvider {
  pub fn as_str(&self) -> &'static str {
    match self {
      StorageProvider::None => "none",
      StorageProvider::GoogleDrive => "google_drive",
    }
  }
}

impl FromStr for StorageProvider {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "none" | "" => Ok(StorageProvider::None),
      "google_drive" => Ok(StorageProvider::GoogleDrive),
      _ => Err(format!("Unknown storage provider: {}", s)),
    }
  }
}

/// Storage configuration for different providers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum StorageConfig {
  None,
  GoogleDrive(GoogleDriveConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveConfig {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub service_account_key: Option<String>, // Base64 encoded key or path (deprecated - use OAuth)
  pub parent_folder_id: Option<String>,
  pub folder_path: Option<String>, // e.g., "Invoices" or "Documents/Invoices"
}
