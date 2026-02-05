use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::CloudStorage;
use async_trait::async_trait;

/// No-operation cloud storage adapter
/// Used when no cloud storage is configured for a company
pub struct NoOpCloudStorage;

impl NoOpCloudStorage {
  pub fn new() -> Self {
    Self
  }
}

impl Default for NoOpCloudStorage {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl CloudStorage for NoOpCloudStorage {
  async fn ensure_invoice_folder(
    &self,
    _company_name: &str,
    _subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    tracing::debug!("NoOpCloudStorage: Skipping folder creation (no cloud storage configured)");
    Ok("local".to_string())
  }

  async fn upload_invoice_pdf(
    &self,
    _company_name: &str,
    _invoice_number: &str,
    local_pdf_path: &str,
    _subfolder_path: &str,
  ) -> Result<String, InvoiceError> {
    tracing::debug!(
      "NoOpCloudStorage: Skipping cloud upload (no cloud storage configured). PDF stored locally at: {}",
      local_pdf_path
    );
    Ok(format!("local:{}", local_pdf_path))
  }
}
