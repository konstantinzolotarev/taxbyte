use std::sync::Arc;
use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct DeleteReportCommand {
  pub report_id: Uuid,
}

pub struct DeleteReportUseCase {
  report_service: Arc<ReportService>,
}

impl DeleteReportUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(&self, command: DeleteReportCommand) -> Result<(), ReportError> {
    self.report_service.delete_report(command.report_id).await
  }
}
