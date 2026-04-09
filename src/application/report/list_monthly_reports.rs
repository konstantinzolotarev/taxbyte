use std::sync::Arc;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct ListMonthlyReportsCommand {
  pub company_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportSummary {
  pub id: Uuid,
  pub month: u32,
  pub year: i32,
  pub status: String,
  pub bank_account_iban: Option<String>,
  pub total_incoming: Decimal,
  pub total_outgoing: Decimal,
  pub transaction_count: i32,
  pub matched_count: i32,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct ListMonthlyReportsResponse {
  pub reports: Vec<ReportSummary>,
}

pub struct ListMonthlyReportsUseCase {
  report_service: Arc<ReportService>,
}

impl ListMonthlyReportsUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(
    &self,
    command: ListMonthlyReportsCommand,
  ) -> Result<ListMonthlyReportsResponse, ReportError> {
    let reports = self
      .report_service
      .get_company_reports(command.company_id)
      .await?;

    let summaries = reports
      .into_iter()
      .map(|r| ReportSummary {
        id: r.id,
        month: r.month,
        year: r.year,
        status: r.status.as_str().to_string(),
        bank_account_iban: r.bank_account_iban,
        total_incoming: r.total_incoming,
        total_outgoing: r.total_outgoing,
        transaction_count: r.transaction_count,
        matched_count: r.matched_count,
        created_at: r.created_at,
      })
      .collect();

    Ok(ListMonthlyReportsResponse { reports: summaries })
  }
}
