use async_trait::async_trait;
use std::path::PathBuf;
use tokio::process::Command;
use uuid::Uuid;

use crate::application::invoice::get_invoice_details::InvoiceDetailsResponse;
use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::PdfGenerator;

pub struct WkHtmlToPdfGenerator {
  pdf_output_dir: PathBuf,
  wkhtmltopdf_path: String,
  server_base_url: String,
}

impl WkHtmlToPdfGenerator {
  pub fn new(
    pdf_output_dir: PathBuf,
    wkhtmltopdf_path: Option<String>,
    server_base_url: String,
  ) -> Self {
    // Create output directory if doesn't exist
    std::fs::create_dir_all(&pdf_output_dir).ok();

    let wkhtmltopdf_path = wkhtmltopdf_path.unwrap_or_else(|| "wkhtmltopdf".to_string());

    Self {
      pdf_output_dir,
      wkhtmltopdf_path,
      server_base_url,
    }
  }

  async fn verify_wkhtmltopdf_installed(&self) -> Result<(), InvoiceError> {
    let output = Command::new(&self.wkhtmltopdf_path)
      .arg("--version")
      .output()
      .await
      .map_err(|e| {
        InvoiceError::PdfGenerationFailed(format!(
          "wkhtmltopdf not found: {}. Please install wkhtmltopdf.",
          e
        ))
      })?;

    if !output.status.success() {
      return Err(InvoiceError::PdfGenerationFailed(
        "wkhtmltopdf is not working correctly".to_string(),
      ));
    }

    Ok(())
  }
}

#[async_trait]
impl PdfGenerator for WkHtmlToPdfGenerator {
  async fn generate_invoice_pdf(
    &self,
    invoice_id: Uuid,
    _invoice_data: &InvoiceDetailsResponse,
  ) -> Result<String, InvoiceError> {
    // Verify wkhtmltopdf is available
    self.verify_wkhtmltopdf_installed().await?;

    // 1. Build URL for invoice HTML view
    let invoice_url = format!("{}/invoices/{}/html", self.server_base_url, invoice_id);
    tracing::info!("Generating PDF from URL: {}", invoice_url);

    // 2. Generate PDF using wkhtmltopdf from URL
    let pdf_filename = format!("{}.pdf", invoice_id);
    let output_path = self.pdf_output_dir.join(&pdf_filename);

    let output = Command::new(&self.wkhtmltopdf_path)
      .args([
        "--page-size",
        "A4",
        "--margin-top",
        "10mm",
        "--margin-bottom",
        "10mm",
        "--margin-left",
        "10mm",
        "--margin-right",
        "10mm",
        "--quiet", // Suppress verbose output
        &invoice_url,
        output_path.to_str().unwrap(),
      ])
      .output()
      .await
      .map_err(|e| {
        InvoiceError::PdfGenerationFailed(format!("wkhtmltopdf execution failed: {}", e))
      })?;

    // 5. Check if PDF generation succeeded
    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr);
      return Err(InvoiceError::PdfGenerationFailed(format!(
        "wkhtmltopdf failed: {}",
        stderr
      )));
    }

    // 6. Verify PDF was created
    if !output_path.exists() {
      return Err(InvoiceError::PdfGenerationFailed(
        "PDF file was not created".to_string(),
      ));
    }

    Ok(output_path.to_string_lossy().to_string())
  }
}
