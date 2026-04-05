use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::domain::report::{errors::ReportError, ports::ReportCloudStorage};

/// No-op implementation for when Drive is not configured
pub struct NoOpReportCloudStorage;

#[async_trait]
impl ReportCloudStorage for NoOpReportCloudStorage {
  async fn create_folder(&self, _parent_id: &str, _name: &str) -> Result<String, ReportError> {
    Err(ReportError::CloudStorage(
      "Cloud storage not configured. Set reports_folder_id in company settings.".to_string(),
    ))
  }

  async fn upload_file(
    &self,
    _folder_id: &str,
    _file_name: &str,
    _local_path: &str,
    _mime_type: &str,
  ) -> Result<String, ReportError> {
    Err(ReportError::CloudStorage(
      "Cloud storage not configured".to_string(),
    ))
  }
}

/// Google Drive adapter for report folder/file operations
pub struct ReportDriveAdapter {
  client: Client,
  client_id: String,
  client_secret: String,
  refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
  access_token: String,
}

#[derive(Debug, Deserialize)]
struct DriveFile {
  id: String,
}

impl ReportDriveAdapter {
  pub fn new(client_id: String, client_secret: String, refresh_token: String) -> Self {
    Self {
      client: Client::new(),
      client_id,
      client_secret,
      refresh_token,
    }
  }

  async fn get_access_token(&self) -> Result<String, ReportError> {
    let response = self
      .client
      .post("https://oauth2.googleapis.com/token")
      .form(&[
        ("client_id", self.client_id.as_str()),
        ("client_secret", self.client_secret.as_str()),
        ("refresh_token", self.refresh_token.as_str()),
        ("grant_type", "refresh_token"),
      ])
      .send()
      .await
      .map_err(|e| ReportError::CloudStorage(format!("Token refresh failed: {}", e)))?;

    if !response.status().is_success() {
      let body = response.text().await.unwrap_or_default();
      return Err(ReportError::CloudStorage(format!(
        "Token refresh failed: {}",
        body
      )));
    }

    let token: TokenResponse = response
      .json()
      .await
      .map_err(|e| ReportError::CloudStorage(format!("Token parse failed: {}", e)))?;

    Ok(token.access_token)
  }
}

#[async_trait]
impl ReportCloudStorage for ReportDriveAdapter {
  async fn create_folder(&self, parent_id: &str, name: &str) -> Result<String, ReportError> {
    let token = self.get_access_token().await?;

    let metadata = json!({
        "name": name,
        "mimeType": "application/vnd.google-apps.folder",
        "parents": [parent_id]
    });

    let response = self
      .client
      .post("https://www.googleapis.com/drive/v3/files")
      .bearer_auth(&token)
      .json(&metadata)
      .send()
      .await
      .map_err(|e| ReportError::CloudStorage(format!("Create folder failed: {}", e)))?;

    if !response.status().is_success() {
      let body = response.text().await.unwrap_or_default();
      return Err(ReportError::CloudStorage(format!(
        "Create folder failed: {}",
        body
      )));
    }

    let file: DriveFile = response
      .json()
      .await
      .map_err(|e| ReportError::CloudStorage(format!("Parse response failed: {}", e)))?;

    Ok(file.id)
  }

  async fn upload_file(
    &self,
    folder_id: &str,
    file_name: &str,
    local_path: &str,
    mime_type: &str,
  ) -> Result<String, ReportError> {
    let token = self.get_access_token().await?;

    let file_bytes = tokio::fs::read(local_path)
      .await
      .map_err(|e| ReportError::FileError(format!("Failed to read {}: {}", local_path, e)))?;

    let metadata = json!({
        "name": file_name,
        "parents": [folder_id]
    });

    let metadata_part = reqwest::multipart::Part::text(metadata.to_string())
      .mime_str("application/json")
      .map_err(|e| ReportError::CloudStorage(e.to_string()))?;

    let file_part = reqwest::multipart::Part::bytes(file_bytes)
      .mime_str(mime_type)
      .map_err(|e| ReportError::CloudStorage(e.to_string()))?;

    let form = reqwest::multipart::Form::new()
      .part("metadata", metadata_part)
      .part("file", file_part);

    let response = self
      .client
      .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
      .bearer_auth(&token)
      .multipart(form)
      .send()
      .await
      .map_err(|e| ReportError::CloudStorage(format!("Upload failed: {}", e)))?;

    if !response.status().is_success() {
      let body = response.text().await.unwrap_or_default();
      return Err(ReportError::CloudStorage(format!(
        "Upload failed: {}",
        body
      )));
    }

    let file: DriveFile = response
      .json()
      .await
      .map_err(|e| ReportError::CloudStorage(format!("Parse response failed: {}", e)))?;

    Ok(file.id)
  }
}
