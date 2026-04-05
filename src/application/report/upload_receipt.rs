use std::sync::Arc;

use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct UploadReceiptCommand {
  pub transaction_id: Uuid,
  pub receipt_path: String,
}

pub struct UploadReceiptUseCase {
  report_service: Arc<ReportService>,
}

impl UploadReceiptUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(&self, command: UploadReceiptCommand) -> Result<(), ReportError> {
    self
      .report_service
      .update_receipt_path(command.transaction_id, Some(command.receipt_path))
      .await
  }
}
