use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::application::company::ConnectGoogleDriveUseCase;
use crate::application::invoice::get_invoice_details::{
  GetInvoiceDetailsCommand, GetInvoiceDetailsUseCase,
};
use crate::domain::company::CompanyRepository;
use crate::domain::invoice::errors::InvoiceError;
use crate::domain::invoice::ports::PdfGenerator;
use crate::domain::invoice::{InvoiceService, InvoiceStatus};
use crate::infrastructure::cloud::CloudStorageFactory;
use crate::infrastructure::config::Config;
use crate::infrastructure::security::AesTokenEncryption;

#[derive(Debug, Deserialize)]
pub struct ChangeInvoiceStatusCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
  pub new_status: String,
}

#[derive(Debug, Serialize)]
pub struct ChangeInvoiceStatusResponse {
  pub invoice_id: Uuid,
  pub status: String,
  pub pdf_path: Option<String>,
}

pub struct ChangeInvoiceStatusUseCase {
  invoice_service: Arc<InvoiceService>,
  pdf_generator: Arc<dyn PdfGenerator>,
  get_invoice_details: Arc<GetInvoiceDetailsUseCase>,
  company_repo: Arc<dyn CompanyRepository>,
  token_encryption: Arc<AesTokenEncryption>,
  connect_google_drive: Arc<ConnectGoogleDriveUseCase>,
  config: Arc<Config>,
}

impl ChangeInvoiceStatusUseCase {
  pub fn new(
    invoice_service: Arc<InvoiceService>,
    pdf_generator: Arc<dyn PdfGenerator>,
    get_invoice_details: Arc<GetInvoiceDetailsUseCase>,
    company_repo: Arc<dyn CompanyRepository>,
    token_encryption: Arc<AesTokenEncryption>,
    connect_google_drive: Arc<ConnectGoogleDriveUseCase>,
    config: Arc<Config>,
  ) -> Self {
    Self {
      invoice_service,
      pdf_generator,
      get_invoice_details,
      company_repo,
      token_encryption,
      connect_google_drive,
      config,
    }
  }

  pub async fn execute(
    &self,
    command: ChangeInvoiceStatusCommand,
  ) -> Result<ChangeInvoiceStatusResponse, InvoiceError> {
    let new_status = InvoiceStatus::from_str(&command.new_status)?;

    // If changing to "Sent", generate PDF and upload to Drive first
    let (pdf_path, drive_file_id) = if new_status == InvoiceStatus::Sent {
      // Get full invoice details for PDF generation
      let invoice_details = self
        .get_invoice_details
        .execute(GetInvoiceDetailsCommand {
          user_id: command.user_id,
          invoice_id: command.invoice_id,
        })
        .await?;

      // Get company's invoice folder path (from DB or use default)
      let invoice_folder_path = invoice_details
        .company
        .invoice_folder_path
        .clone()
        .unwrap_or_else(|| "Invoices".to_string());

      // Generate PDF
      let pdf_path = self
        .pdf_generator
        .generate_invoice_pdf(command.invoice_id, &invoice_details)
        .await?;

      // Fetch full company entity with OAuth tokens
      let company = self
        .company_repo
        .find_by_id(invoice_details.company_id)
        .await
        .map_err(|e| {
          InvoiceError::CloudStorageUploadFailed(format!("Failed to fetch company: {}", e))
        })?
        .ok_or_else(|| InvoiceError::CloudStorageUploadFailed("Company not found".to_string()))?;

      // Extract OAuth credentials from config
      let (oauth_client_id, oauth_client_secret) =
        if let Some(ref drive_config) = self.config.google_drive {
          (
            drive_config.oauth_client_id.as_deref(),
            drive_config.oauth_client_secret.as_deref(),
          )
        } else {
          (None, None)
        };

      // Create cloud storage adapter with OAuth support
      let cloud_storage = CloudStorageFactory::create_with_oauth(
        company.storage_provider.as_ref(),
        company.storage_config.as_ref(),
        &company,
        &self.token_encryption,
        Some(&self.connect_google_drive),
        oauth_client_id,
        oauth_client_secret,
      )
      .await;

      // Upload to cloud storage
      let file_id = cloud_storage
        .upload_invoice_pdf(
          &invoice_details.company.name,
          &invoice_details.invoice_number,
          &pdf_path,
          &invoice_folder_path,
        )
        .await?;

      (Some(pdf_path), Some(file_id))
    } else {
      (None, None)
    };

    // Change invoice status
    let mut invoice = self
      .invoice_service
      .change_invoice_status(command.user_id, command.invoice_id, new_status)
      .await?;

    // Store PDF path and Drive file ID if generated
    if let Some(pdf_path) = &pdf_path {
      invoice = self
        .invoice_service
        .set_invoice_pdf_path(command.invoice_id, pdf_path.clone(), drive_file_id)
        .await?;
    }

    Ok(ChangeInvoiceStatusResponse {
      invoice_id: invoice.id,
      status: invoice.status.as_str().to_string(),
      pdf_path: invoice.pdf_path,
    })
  }
}
