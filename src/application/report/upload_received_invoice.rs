use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::report::{
  entities::ReceivedInvoice, errors::ReportError, services::ReportService,
};

#[derive(Debug)]
pub struct UploadReceivedInvoiceCommand {
  pub company_id: Uuid,
  pub vendor_name: String,
  pub amount: Decimal,
  pub currency: String,
  pub invoice_date: Option<NaiveDate>,
  pub invoice_number: Option<String>,
  pub pdf_path: String,
  pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UploadReceivedInvoiceResponse {
  pub id: Uuid,
  pub vendor_name: String,
  pub amount: Decimal,
  pub created_at: DateTime<Utc>,
}

pub struct UploadReceivedInvoiceUseCase {
  report_service: Arc<ReportService>,
}

impl UploadReceivedInvoiceUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(
    &self,
    command: UploadReceivedInvoiceCommand,
  ) -> Result<UploadReceivedInvoiceResponse, ReportError> {
    if command.vendor_name.trim().is_empty() {
      return Err(ReportError::Validation(
        "Vendor name is required".to_string(),
      ));
    }

    let invoice = ReceivedInvoice::new(
      command.company_id,
      command.vendor_name,
      command.amount,
      command.currency,
      command.invoice_date,
      command.invoice_number,
      command.pdf_path,
      command.notes,
    );

    let created = self.report_service.create_received_invoice(invoice).await?;

    Ok(UploadReceivedInvoiceResponse {
      id: created.id,
      vendor_name: created.vendor_name,
      amount: created.amount,
      created_at: created.created_at,
    })
  }
}
