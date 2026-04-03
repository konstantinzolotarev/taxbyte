use async_trait::async_trait;
use google_drive3::{DriveHub, api::File as DriveFile, hyper, hyper_rustls, oauth2};

use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;

pub struct GoogleDriveAdapter {
  hub: DriveHub<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl GoogleDriveAdapter {
  pub async fn new(service_account_key_path: &str) -> Result<Self, InvoiceError> {
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

    Ok(Self { hub })
  }

  /// Create adapter from JSON key content directly
  pub async fn new_from_json(key_json: &str) -> Result<Self, InvoiceError> {
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

    Ok(Self { hub })
  }
}

#[async_trait]
impl CloudStorage for GoogleDriveAdapter {
  async fn upload_invoice_pdf(
    &self,
    folder_id: &str,
    invoice_number: &str,
    local_pdf_path: &str,
  ) -> Result<String, InvoiceError> {
    // Prepare file metadata
    let filename = format!("{}.pdf", invoice_number);
    let file_metadata = DriveFile {
      name: Some(filename),
      mime_type: Some("application/pdf".to_string()),
      parents: Some(vec![folder_id.to_string()]),
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
