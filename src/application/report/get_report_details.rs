use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::report::{errors::ReportError, services::ReportService};

#[derive(Debug)]
pub struct GetReportDetailsCommand {
  pub report_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionDetail {
  pub id: Uuid,
  pub row_number: i32,
  pub date: NaiveDate,
  pub counterparty_name: Option<String>,
  pub counterparty_account: Option<String>,
  pub direction: String,
  pub amount: Decimal,
  pub reference_number: Option<String>,
  pub description: Option<String>,
  pub currency: String,
  pub registry_code: Option<String>,
  pub matched_invoice_id: Option<Uuid>,
  pub matched_received_invoice_id: Option<Uuid>,
  pub is_matched: bool,
}

#[derive(Debug, Serialize)]
pub struct GetReportDetailsResponse {
  pub id: Uuid,
  pub company_id: Uuid,
  pub month: u32,
  pub year: i32,
  pub status: String,
  pub bank_account_iban: String,
  pub total_incoming: Decimal,
  pub total_outgoing: Decimal,
  pub transaction_count: i32,
  pub matched_count: i32,
  pub drive_folder_id: Option<String>,
  pub transactions: Vec<TransactionDetail>,
  pub created_at: DateTime<Utc>,
}

pub struct GetReportDetailsUseCase {
  report_service: Arc<ReportService>,
}

impl GetReportDetailsUseCase {
  pub fn new(report_service: Arc<ReportService>) -> Self {
    Self { report_service }
  }

  pub async fn execute(
    &self,
    command: GetReportDetailsCommand,
  ) -> Result<GetReportDetailsResponse, ReportError> {
    let (report, transactions) = self
      .report_service
      .get_report_details(command.report_id)
      .await?;

    let tx_details: Vec<TransactionDetail> = transactions
      .into_iter()
      .map(|t| {
        let is_matched = t.is_matched();
        TransactionDetail {
          id: t.id,
          row_number: t.row_number,
          date: t.date,
          counterparty_name: t.counterparty_name,
          counterparty_account: t.counterparty_account,
          direction: t.direction.as_str().to_string(),
          amount: t.amount,
          reference_number: t.reference_number,
          description: t.description,
          currency: t.currency,
          registry_code: t.registry_code,
          matched_invoice_id: t.matched_invoice_id,
          matched_received_invoice_id: t.matched_received_invoice_id,
          is_matched,
        }
      })
      .collect();

    Ok(GetReportDetailsResponse {
      id: report.id,
      company_id: report.company_id,
      month: report.month,
      year: report.year,
      status: report.status.as_str().to_string(),
      bank_account_iban: report.bank_account_iban,
      total_incoming: report.total_incoming,
      total_outgoing: report.total_outgoing,
      transaction_count: report.transaction_count,
      matched_count: report.matched_count,
      drive_folder_id: report.drive_folder_id,
      transactions: tx_details,
      created_at: report.created_at,
    })
  }
}
