use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;

/// Google Drive adapter using OAuth 2.0 user tokens
pub struct GoogleDriveOAuthAdapter {
  client: Client,
  client_id: String,
  client_secret: String,
  refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
  access_token: String,
  #[allow(dead_code)]
  expires_in: u64,
  #[allow(dead_code)]
  token_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DriveFile {
  id: String,
  #[allow(dead_code)]
  name: Option<String>,
}

impl GoogleDriveOAuthAdapter {
  /// Create adapter from OAuth tokens
  pub async fn new(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
  ) -> Result<Self, InvoiceError> {
    Ok(Self {
      client: Client::new(),
      client_id: client_id.to_string(),
      client_secret: client_secret.to_string(),
      refresh_token: refresh_token.to_string(),
    })
  }

  /// Get a fresh access token using the refresh token
  async fn get_access_token(&self) -> Result<String, InvoiceError> {
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
      .map_err(|e| {
        InvoiceError::CloudStorageUploadFailed(format!("Token refresh failed: {}", e))
      })?;

    if !response.status().is_success() {
      let status = response.status();
      let body = response.text().await.unwrap_or_default();
      return Err(InvoiceError::CloudStorageUploadFailed(format!(
        "Token refresh failed with status {}: {}",
        status, body
      )));
    }

    let token_response: TokenResponse = response.json().await.map_err(|e| {
      InvoiceError::CloudStorageUploadFailed(format!("Failed to parse token response: {}", e))
    })?;

    Ok(token_response.access_token)
  }
}

#[async_trait]
impl CloudStorage for GoogleDriveOAuthAdapter {
  async fn upload_invoice_pdf(
    &self,
    folder_id: &str,
    invoice_number: &str,
    local_pdf_path: &str,
  ) -> Result<String, InvoiceError> {
    let access_token = self.get_access_token().await?;

    // Read PDF file
    let pdf_content = tokio::fs::read(local_pdf_path)
      .await
      .map_err(|e| InvoiceError::CloudStorageUploadFailed(format!("Failed to read PDF: {}", e)))?;

    let file_name = format!("{}.pdf", invoice_number);

    // Upload file using multipart
    let metadata = json!({
      "name": file_name,
      "parents": [folder_id]
    });

    let form = reqwest::multipart::Form::new()
      .part(
        "metadata",
        reqwest::multipart::Part::text(metadata.to_string())
          .mime_str("application/json")
          .map_err(|e| InvoiceError::CloudStorageUploadFailed(format!("MIME type error: {}", e)))?,
      )
      .part(
        "file",
        reqwest::multipart::Part::bytes(pdf_content)
          .file_name(file_name.clone())
          .mime_str("application/pdf")
          .map_err(|e| InvoiceError::CloudStorageUploadFailed(format!("MIME type error: {}", e)))?,
      );

    let response = self
      .client
      .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id")
      .bearer_auth(&access_token)
      .multipart(form)
      .send()
      .await
      .map_err(|e| InvoiceError::CloudStorageUploadFailed(format!("Upload failed: {}", e)))?;

    if !response.status().is_success() {
      let status = response.status();
      let body = response.text().await.unwrap_or_default();
      return Err(InvoiceError::CloudStorageUploadFailed(format!(
        "Upload failed with status {}: {}",
        status, body
      )));
    }

    let file: DriveFile = response.json().await.map_err(|e| {
      InvoiceError::CloudStorageUploadFailed(format!("Failed to parse upload response: {}", e))
    })?;

    tracing::info!(
      "Successfully uploaded {} to Google Drive (file ID: {})",
      file_name,
      file.id
    );

    Ok(file.id)
  }
}
