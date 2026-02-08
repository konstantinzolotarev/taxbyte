use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;

/// Google Drive adapter using OAuth 2.0 user tokens
///
/// # Implementation Status: REQUIRES COMPLETION
///
/// This adapter is partially implemented. The OAuth flow (token exchange, storage, refresh)
/// is fully functional, but the Drive API integration has dependency conflicts that need
/// resolution.
///
/// ## What Works:
/// - ✅ OAuth 2.0 consent flow (initiate, callback)
/// - ✅ Token storage (encrypted in database)
/// - ✅ Token refresh mechanism
/// - ✅ Mock OAuth for development (MOCK_OAUTH=true)
/// - ✅ UI integration (connect/disconnect/test buttons)
///
/// ## What Needs Completion:
/// - ❌ DriveHub initialization with OAuth tokens (dependency version conflicts)
/// - ❌ File upload with user credentials
/// - ❌ Folder creation with user credentials
///
/// ## Technical Issue:
/// The `google-drive3` crate (v5.0.5) uses `yup-oauth2` v9.0 and `hyper` v0.14.
/// Creating an `AuthorizedUserAuthenticator` requires matching all dependency versions
/// precisely, which conflicts with other crates in the dependency tree.
///
/// ## Recommended Solution Options:
///
/// ### Option 1: Use google-drive3's built-in OAuth (Recommended)
/// Wait for `google-drive3` to expose a simpler OAuth interface, or use
/// `InstalledFlowAuthenticator` (requires user to complete auth flow each time).
///
/// ### Option 2: Custom HTTP client
/// Bypass `google-drive3` entirely and make direct REST API calls to Google Drive
/// using `reqwest` with OAuth bearer tokens. This gives full control but requires
/// implementing all Drive API methods manually.
///
/// ### Option 3: Upgrade google-drive3
/// Wait for or contribute to a `google-drive3` v6.x that uses newer dependencies.
///
/// ## For Now:
/// - OAuth flow works end-to-end
/// - Tokens are stored securely
/// - Invoice uploads will fall back to service account (if configured)
/// - Clear error message shown to users when OAuth upload attempted
///
/// ## Code Structure:
/// The placeholder implementation below maintains the correct interface so the
/// rest of the system compiles and runs. When dependency issues are resolved,
/// the implementation can be completed using the research findings documented
/// in the commit history.
pub struct GoogleDriveOAuthAdapter {
  client: Client,
  client_id: String,
  client_secret: String,
  refresh_token: String,
  parent_folder_id: Option<String>,
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
    parent_folder_id: Option<String>,
  ) -> Result<Self, InvoiceError> {
    Ok(Self {
      client: Client::new(),
      client_id: client_id.to_string(),
      client_secret: client_secret.to_string(),
      refresh_token: refresh_token.to_string(),
      parent_folder_id,
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

  /// Find folder by name in a parent folder
  async fn find_folder(
    &self,
    access_token: &str,
    folder_name: &str,
    parent_id: Option<&str>,
  ) -> Result<Option<String>, InvoiceError> {
    let mut query = format!(
      "name='{}' and mimeType='application/vnd.google-apps.folder' and trashed=false",
      folder_name
    );
    if let Some(parent) = parent_id {
      query.push_str(&format!(" and '{}' in parents", parent));
    }

    let response = self
      .client
      .get("https://www.googleapis.com/drive/v3/files")
      .bearer_auth(access_token)
      .query(&[("q", query.as_str()), ("fields", "files(id, name)")])
      .send()
      .await
      .map_err(|e| {
        InvoiceError::CloudStorageUploadFailed(format!("Folder search failed: {}", e))
      })?;

    if !response.status().is_success() {
      return Err(InvoiceError::CloudStorageUploadFailed(format!(
        "Folder search failed: {}",
        response.status()
      )));
    }

    #[derive(Deserialize)]
    struct FilesResponse {
      files: Vec<DriveFile>,
    }

    let files_response: FilesResponse = response.json().await.map_err(|e| {
      InvoiceError::CloudStorageUploadFailed(format!("Failed to parse search response: {}", e))
    })?;

    Ok(files_response.files.first().map(|f| f.id.clone()))
  }

  /// Create a folder
  async fn create_folder(
    &self,
    access_token: &str,
    folder_name: &str,
    parent_id: Option<&str>,
  ) -> Result<String, InvoiceError> {
    let mut metadata = json!({
      "name": folder_name,
      "mimeType": "application/vnd.google-apps.folder"
    });

    if let Some(parent) = parent_id {
      metadata["parents"] = json!([parent]);
    }

    let response = self
      .client
      .post("https://www.googleapis.com/drive/v3/files")
      .bearer_auth(access_token)
      .query(&[("fields", "id")])
      .json(&metadata)
      .send()
      .await
      .map_err(|e| {
        InvoiceError::CloudStorageUploadFailed(format!("Folder creation failed: {}", e))
      })?;

    if !response.status().is_success() {
      return Err(InvoiceError::CloudStorageUploadFailed(format!(
        "Folder creation failed: {}",
        response.status()
      )));
    }

    let folder: DriveFile = response.json().await.map_err(|e| {
      InvoiceError::CloudStorageUploadFailed(format!("Failed to parse folder response: {}", e))
    })?;

    Ok(folder.id)
  }
}

#[async_trait]
impl CloudStorage for GoogleDriveOAuthAdapter {
  async fn ensure_invoice_folder(
    &self,
    _company_name: &str,
    subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    let access_token = self.get_access_token().await?;

    // Start from parent folder if specified
    let current_folder_id = self.parent_folder_id.clone();

    // Create/find the invoice folder directly (skip company folder)
    // Use subfolder_path if specified, otherwise use company name
    let folder_name = if !subfolder_path.is_empty() {
      subfolder_path
    } else {
      _company_name
    };

    let folder_id = if let Some(existing_id) = self
      .find_folder(&access_token, folder_name, current_folder_id.as_deref())
      .await?
    {
      existing_id
    } else {
      self
        .create_folder(&access_token, folder_name, current_folder_id.as_deref())
        .await?
    };

    Ok(folder_id)
  }

  async fn upload_invoice_pdf(
    &self,
    company_name: &str,
    invoice_number: &str,
    local_pdf_path: &str,
    subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    let access_token = self.get_access_token().await?;

    // Ensure folder exists
    let folder_id = self
      .ensure_invoice_folder(company_name, subfolder_path)
      .await?;

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
