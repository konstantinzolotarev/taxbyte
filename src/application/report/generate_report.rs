use std::sync::Arc;
use uuid::Uuid;

use crate::domain::company::ports::CompanyRepository;
use crate::domain::invoice::ports::InvoiceRepository;
use crate::domain::report::{
  errors::ReportError,
  ports::ReportCloudStorage,
  services::ReportService,
  value_objects::{ReportMonth, ReportStatus, TransactionDirection},
};

#[derive(Debug)]
pub struct GenerateReportCommand {
  pub report_id: Uuid,
  pub company_id: Uuid,
}

pub struct GenerateReportUseCase {
  report_service: Arc<ReportService>,
  company_repo: Arc<dyn CompanyRepository>,
  invoice_repo: Arc<dyn InvoiceRepository>,
  cloud_storage: Arc<dyn ReportCloudStorage>,
}

impl GenerateReportUseCase {
  pub fn new(
    report_service: Arc<ReportService>,
    company_repo: Arc<dyn CompanyRepository>,
    invoice_repo: Arc<dyn InvoiceRepository>,
    cloud_storage: Arc<dyn ReportCloudStorage>,
  ) -> Self {
    Self {
      report_service,
      company_repo,
      invoice_repo,
      cloud_storage,
    }
  }

  pub async fn execute(&self, command: GenerateReportCommand) -> Result<String, ReportError> {
    let (report, transactions) = self
      .report_service
      .get_report_details(command.report_id)
      .await?;

    if report.status != ReportStatus::Draft {
      return Err(ReportError::NotDraft);
    }

    let matched: Vec<_> = transactions.iter().filter(|t| t.is_matched()).collect();
    if matched.is_empty() {
      return Err(ReportError::NoMatchedTransactions);
    }

    // Get company's reports folder
    let company = self
      .company_repo
      .find_by_id(command.company_id)
      .await
      .map_err(|e| ReportError::CloudStorage(e.to_string()))?
      .ok_or_else(|| ReportError::CloudStorage("Company not found".to_string()))?;

    let reports_folder_id = company.reports_folder_id.ok_or_else(|| {
      ReportError::CloudStorage(
        "Reports folder ID not configured. Set it in company settings.".to_string(),
      )
    })?;

    // Create MM.YYYY folder
    let period = ReportMonth::new(report.month, report.year)?;
    let month_folder_id = self
      .cloud_storage
      .create_folder(&reports_folder_id, &period.folder_name())
      .await?;

    // Create incoming/ and outcoming/ subfolders
    let incoming_folder_id = self
      .cloud_storage
      .create_folder(&month_folder_id, "incoming")
      .await?;
    let outcoming_folder_id = self
      .cloud_storage
      .create_folder(&month_folder_id, "outcoming")
      .await?;

    // Upload matched PDFs
    for tx in &matched {
      match tx.direction {
        TransactionDirection::Credit => {
          // Incoming money = issued invoice paid
          if let Some(invoice_id) = tx.matched_invoice_id {
            if let Ok(Some(invoice)) = self.invoice_repo.find_by_id(invoice_id).await {
              if let Some(pdf_path) = &invoice.pdf_path {
                let file_name = format!(
                  "{} - {}.pdf",
                  tx.date.format("%Y-%m-%d"),
                  tx.counterparty_name.as_deref().unwrap_or("unknown")
                );
                let _ = self
                  .cloud_storage
                  .upload_file(&incoming_folder_id, &file_name, pdf_path, "application/pdf")
                  .await;
              }
            }
          }
        }
        TransactionDirection::Debit => {
          // Outgoing money = received invoice (bill) paid
          if let Some(received_id) = tx.matched_received_invoice_id {
            if let Ok(invoice) = self.report_service.get_received_invoice(received_id).await {
              let file_name = format!(
                "{} - {}.pdf",
                tx.date.format("%Y-%m-%d"),
                tx.counterparty_name
                  .as_deref()
                  .unwrap_or(&invoice.vendor_name)
              );
              let _ = self
                .cloud_storage
                .upload_file(
                  &outcoming_folder_id,
                  &file_name,
                  &invoice.pdf_path,
                  "application/pdf",
                )
                .await;
            }
          }
        }
      }
    }

    // Mark report as generated
    self
      .report_service
      .mark_generated(command.report_id, month_folder_id.clone())
      .await?;

    Ok(month_folder_id)
  }
}
