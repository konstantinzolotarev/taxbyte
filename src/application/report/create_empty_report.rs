use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::report::{
  errors::ReportError, services::ReportService, value_objects::ReportMonth,
};

#[derive(Debug)]
pub struct CreateEmptyReportCommand {
  pub company_id: Uuid,
  pub month: u32,
  pub year: i32,
}

#[derive(Debug, Clone)]
pub struct CreateEmptyReportResponse {
  pub report_id: Uuid,
  pub month: u32,
  pub year: i32,
  pub status: String,
  pub created_at: DateTime<Utc>,
}

pub struct CreateEmptyReportUseCase {
  report_service: Arc<ReportService>,
}

impl CreateEmptyReportUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(
    &self,
    command: CreateEmptyReportCommand,
  ) -> Result<CreateEmptyReportResponse, ReportError> {
    let period = ReportMonth::new(command.month, command.year)?;

    let report = self
      .report_service
      .create_empty_report(command.company_id, period)
      .await?;

    Ok(CreateEmptyReportResponse {
      report_id: report.id,
      month: report.month,
      year: report.year,
      status: report.status.as_str().to_string(),
      created_at: report.created_at,
    })
  }
}
