use std::sync::Arc;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::domain::report::{
  errors::ReportError, ports::BankStatementParser, services::ReportService,
  value_objects::ReportMonth,
};

#[derive(Debug)]
pub struct ImportBankStatementCommand {
  pub company_id: Uuid,
  pub month: u32,
  pub year: i32,
  pub csv_content: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ImportBankStatementResponse {
  pub report_id: Uuid,
  pub month: u32,
  pub year: i32,
  pub transaction_count: i32,
  pub total_incoming: Decimal,
  pub total_outgoing: Decimal,
  pub created_at: DateTime<Utc>,
}

pub struct ImportBankStatementUseCase {
  report_service: Arc<ReportService>,
  parser: Arc<dyn BankStatementParser>,
}

impl ImportBankStatementUseCase {
  pub fn new(report_service: Arc<ReportService>, parser: Arc<dyn BankStatementParser>) -> Self {
    Self {
      report_service,
      parser,
    }
  }

  pub async fn execute(
    &self,
    command: ImportBankStatementCommand,
  ) -> Result<ImportBankStatementResponse, ReportError> {
    let period = ReportMonth::new(command.month, command.year)?;

    let transactions = self.parser.parse(&command.csv_content)?;

    let report = self
      .report_service
      .import_bank_statement(command.company_id, period, transactions)
      .await?;

    Ok(ImportBankStatementResponse {
      report_id: report.id,
      month: report.month,
      year: report.year,
      transaction_count: report.transaction_count,
      total_incoming: report.total_incoming,
      total_outgoing: report.total_outgoing,
      created_at: report.created_at,
    })
  }
}
