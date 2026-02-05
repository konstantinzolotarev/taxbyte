use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Storage provider type for cloud uploads
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StorageProvider {
  #[default]
  None,
  GoogleDrive,
  S3,
}

impl StorageProvider {
  pub fn as_str(&self) -> &'static str {
    match self {
      StorageProvider::None => "none",
      StorageProvider::GoogleDrive => "google_drive",
      StorageProvider::S3 => "s3",
    }
  }
}

impl FromStr for StorageProvider {
  type Err = String;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "none" | "" => Ok(StorageProvider::None),
      "google_drive" => Ok(StorageProvider::GoogleDrive),
      "s3" => Ok(StorageProvider::S3),
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
  S3(S3Config),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveConfig {
  pub service_account_key: String, // Base64 encoded key or path
  pub parent_folder_id: Option<String>,
  pub folder_path: Option<String>, // e.g., "Invoices" or "Documents/Invoices"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
  pub bucket: String,
  pub region: String,
  pub access_key_id: String,
  pub secret_access_key: String,
  pub prefix: Option<String>, // e.g., "invoices/" or "company-name/invoices/"
}
