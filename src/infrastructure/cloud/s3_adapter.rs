use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;
use async_trait::async_trait;

/// Amazon S3 cloud storage adapter
/// TODO: Implement actual S3 integration
pub struct S3Adapter {
  bucket: String,
  _region: String, // TODO: Use when implementing actual S3 client
  prefix: Option<String>,
}

impl S3Adapter {
  pub async fn new(
    bucket: &str,
    region: &str,
    _access_key_id: &str,
    _secret_access_key: &str,
    prefix: Option<String>,
  ) -> Result<Self, InvoiceError> {
    // TODO: Initialize AWS S3 client
    tracing::info!(
      "S3 adapter created for bucket: {}, region: {}",
      bucket,
      region
    );

    Ok(Self {
      bucket: bucket.to_string(),
      _region: region.to_string(),
      prefix,
    })
  }
}

#[async_trait]
impl CloudStorage for S3Adapter {
  async fn ensure_invoice_folder(
    &self,
    company_name: &str,
    subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    // S3 doesn't have "folders" - just key prefixes
    let folder_key = format!(
      "{}{}/{}",
      self.prefix.as_deref().unwrap_or(""),
      company_name,
      subfolder_path
    );
    tracing::debug!("S3: Using folder key: {}", folder_key);
    Ok(folder_key)
  }

  async fn upload_invoice_pdf(
    &self,
    company_name: &str,
    invoice_number: &str,
    local_pdf_path: &str,
    subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    // TODO: Implement actual S3 upload
    let key = format!(
      "{}{}/{}/{}.pdf",
      self.prefix.as_deref().unwrap_or(""),
      company_name,
      subfolder_path,
      invoice_number
    );

    tracing::info!(
      "S3: Would upload {} to s3://{}/{}",
      local_pdf_path,
      self.bucket,
      key
    );

    // For now, return a placeholder S3 URL
    Ok(format!("s3://{}/{}", self.bucket, key))
  }
}
