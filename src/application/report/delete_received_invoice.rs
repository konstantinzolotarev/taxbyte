use std::sync::Arc;
use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct DeleteReceivedInvoiceCommand {
  pub id: Uuid,
}

pub struct DeleteReceivedInvoiceUseCase {
  report_service: Arc<ReportService>,
}

impl DeleteReceivedInvoiceUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(&self, command: DeleteReceivedInvoiceCommand) -> Result<(), ReportError> {
    let pdf_path = self
      .report_service
      .delete_received_invoice(command.id)
      .await?;

    // Delete the PDF file from disk
    if let Err(e) = tokio::fs::remove_file(&pdf_path).await {
      tracing::warn!("Failed to delete PDF file {}: {}", pdf_path, e);
    }

    Ok(())
  }
}
