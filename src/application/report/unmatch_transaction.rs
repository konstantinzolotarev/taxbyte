use std::sync::Arc;
use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct UnmatchTransactionCommand {
  pub transaction_id: Uuid,
}

pub struct UnmatchTransactionUseCase {
  report_service: Arc<ReportService>,
}

impl UnmatchTransactionUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(&self, command: UnmatchTransactionCommand) -> Result<(), ReportError> {
    self
      .report_service
      .unmatch_transaction(command.transaction_id)
      .await
  }
}
