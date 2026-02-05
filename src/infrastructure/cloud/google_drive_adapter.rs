use async_trait::async_trait;
use google_drive3::{DriveHub, api::File as DriveFile, hyper, hyper_rustls, oauth2};

use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;

pub struct GoogleDriveAdapter {
  hub: DriveHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
  parent_folder_id: Option<String>,
}

impl GoogleDriveAdapter {
  pub async fn new(
    service_account_key_path: &str,
    parent_folder_id: Option<String>,
  ) -> Result<Self, InvoiceError> {
    // Load service account credentials
    let key_content = std::fs::read_to_string(service_account_key_path)
      .map_err(|e| InvoiceError::CloudStorageAuthFailed(format!("Key file read failed: {}", e)))?;

    let key: oauth2::ServiceAccountKey = serde_json::from_str(&key_content)
      .map_err(|e| InvoiceError::CloudStorageAuthFailed(format!("Key parse failed: {}", e)))?;

    // Create authenticator
    let auth = oauth2::ServiceAccountAuthenticator::builder(key)
      .build()
      .await
      .map_err(|e| InvoiceError::CloudStorageAuthFailed(format!("Auth build failed: {}", e)))?;

    // Create Drive hub
    let https = hyper_rustls::HttpsConnectorBuilder::new()
      .with_native_roots()
      .unwrap()
      .https_only()
      .enable_http1()
      .build();

    let client = hyper::Client::builder().build(https);
    let hub = DriveHub::new(client, auth);

    Ok(Self {
      hub,
      parent_folder_id,
    })
  }

  /// Create adapter from JSON key content directly
  pub async fn new_from_json(
    key_json: &str,
    parent_folder_id: Option<String>,
  ) -> Result<Self, InvoiceError> {
    let key: oauth2::ServiceAccountKey = serde_json::from_str(key_json)
      .map_err(|e| InvoiceError::CloudStorageAuthFailed(format!("Key parse failed: {}", e)))?;

    let auth = oauth2::ServiceAccountAuthenticator::builder(key)
      .build()
      .await
      .map_err(|e| InvoiceError::CloudStorageAuthFailed(format!("Auth build failed: {}", e)))?;

    let https = hyper_rustls::HttpsConnectorBuilder::new()
      .with_native_roots()
      .unwrap()
      .https_only()
      .enable_http1()
      .build();

    let client = hyper::Client::builder().build(https);
    let hub = DriveHub::new(client, auth);

    Ok(Self {
      hub,
      parent_folder_id,
    })
  }

  /// Ensure a folder exists, creating it if necessary
  /// Returns the folder ID
  async fn ensure_folder(
    &self,
    folder_name: &str,
    parent_id: Option<&str>,
  ) -> Result<String, InvoiceError> {
    // Search for existing folder
    let query = format!(
      "name='{}' and mimeType='application/vnd.google-apps.folder' and trashed=false{}",
      folder_name,
      parent_id
        .map(|id| format!(" and '{}' in parents", id))
        .unwrap_or_default()
    );

    let result = self
      .hub
      .files()
      .list()
      .q(&query)
      .page_size(1)
      .doit()
      .await
      .map_err(|e| {
        InvoiceError::CloudStorageUploadFailed(format!("Folder search failed: {}", e))
      })?;

    // If folder exists, return its ID
    if let Some(files) = result.1.files {
      if let Some(file) = files.into_iter().next() {
        if let Some(id) = file.id {
          return Ok(id);
        }
      }
    }

    // Create new folder
    let folder_metadata = DriveFile {
      name: Some(folder_name.to_string()),
      mime_type: Some("application/vnd.google-apps.folder".to_string()),
      parents: parent_id.map(|id| vec![id.to_string()]),
      ..Default::default()
    };

    let result = self
      .hub
      .files()
      .create(folder_metadata)
      .upload(
        std::io::empty(),
        "application/vnd.google-apps.folder".parse().unwrap(),
      )
      .await
      .map_err(|e| {
        InvoiceError::CloudStorageUploadFailed(format!("Folder creation failed: {}", e))
      })?;

    result
      .1
      .id
      .ok_or_else(|| InvoiceError::CloudStorageUploadFailed("No folder ID returned".to_string()))
  }
}

#[async_trait]
impl CloudStorage for GoogleDriveAdapter {
  async fn ensure_invoice_folder(
    &self,
    company_name: &str,
    subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    // Step 1: Ensure company folder exists
    let company_folder_id = self
      .ensure_folder(company_name, self.parent_folder_id.as_deref())
      .await?;

    // Step 2: Create nested subfolders if needed
    // Split path by '/' and create each folder in sequence
    let subfolder_parts: Vec<&str> = subfolder_path
      .trim_matches('/')
      .split('/')
      .filter(|s| !s.is_empty())
      .collect();

    let mut current_parent_id = company_folder_id;

    for folder_name in subfolder_parts {
      current_parent_id = self
        .ensure_folder(folder_name, Some(&current_parent_id))
        .await?;
    }

    Ok(current_parent_id)
  }

  async fn upload_invoice_pdf(
    &self,
    company_name: &str,
    invoice_number: &str,
    local_pdf_path: &str,
    subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    // Ensure nested folder structure exists
    let folder_id = self
      .ensure_invoice_folder(company_name, subfolder_path)
      .await?;

    // Prepare file metadata
    let filename = format!("{}.pdf", invoice_number);
    let file_metadata = DriveFile {
      name: Some(filename),
      mime_type: Some("application/pdf".to_string()),
      parents: Some(vec![folder_id]),
      ..Default::default()
    };

    // Read file content
    let file_content = tokio::fs::read(local_pdf_path)
      .await
      .map_err(|e| InvoiceError::CloudStorageUploadFailed(format!("File read failed: {}", e)))?;

    // Upload file using simple upload
    let result = self
      .hub
      .files()
      .create(file_metadata)
      .upload(
        std::io::Cursor::new(file_content),
        "application/pdf".parse().unwrap(),
      )
      .await
      .map_err(|e| InvoiceError::CloudStorageUploadFailed(format!("Upload failed: {}", e)))?;

    result
      .1
      .id
      .ok_or_else(|| InvoiceError::CloudStorageUploadFailed("No file ID returned".to_string()))
  }
}
