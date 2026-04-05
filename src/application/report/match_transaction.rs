use std::sync::Arc;
use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct MatchTransactionCommand {
  pub transaction_id: Uuid,
  pub invoice_id: Option<Uuid>,
  pub received_invoice_id: Option<Uuid>,
}

pub struct MatchTransactionUseCase {
  report_service: Arc<ReportService>,
}

impl MatchTransactionUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(&self, command: MatchTransactionCommand) -> Result<(), ReportError> {
    self
      .report_service
      .match_transaction(
        command.transaction_id,
        command.invoice_id,
        command.received_invoice_id,
      )
      .await
  }
}
