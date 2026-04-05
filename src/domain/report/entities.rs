use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::value_objects::{ReportStatus, TransactionDirection};

/// Monthly report representing one imported bank statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyReport {
  pub id: Uuid,
  pub company_id: Uuid,
  pub month: u32,
  pub year: i32,
  pub status: ReportStatus,
  pub bank_account_iban: String,
  pub total_incoming: Decimal,
  pub total_outgoing: Decimal,
  pub transaction_count: i32,
  pub matched_count: i32,
  pub drive_folder_id: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl MonthlyReport {
  pub fn new(company_id: Uuid, month: u32, year: i32, bank_account_iban: String) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4(),
      company_id,
      month,
      year,
      status: ReportStatus::Draft,
      bank_account_iban,
      total_incoming: Decimal::ZERO,
      total_outgoing: Decimal::ZERO,
      transaction_count: 0,
      matched_count: 0,
      drive_folder_id: None,
      created_at: now,
      updated_at: now,
    }
  }
}

/// One row from a bank statement CSV
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransaction {
  pub id: Uuid,
  pub report_id: Uuid,
  pub row_number: i32,
  pub date: NaiveDate,
  pub counterparty_name: Option<String>,
  pub counterparty_account: Option<String>,
  pub direction: TransactionDirection,
  pub amount: Decimal,
  pub reference_number: Option<String>,
  pub description: Option<String>,
  pub currency: String,
  pub registry_code: Option<String>,
  pub matched_invoice_id: Option<Uuid>,
  pub matched_received_invoice_id: Option<Uuid>,
  pub receipt_path: Option<String>,
}

impl BankTransaction {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    report_id: Uuid,
    row_number: i32,
    date: NaiveDate,
    counterparty_name: Option<String>,
    counterparty_account: Option<String>,
    direction: TransactionDirection,
    amount: Decimal,
    reference_number: Option<String>,
    description: Option<String>,
    currency: String,
    registry_code: Option<String>,
  ) -> Self {
    Self {
      id: Uuid::new_v4(),
      report_id,
      row_number,
      date,
      counterparty_name,
      counterparty_account,
      direction,
      amount,
      reference_number,
      description,
      currency,
      registry_code,
      matched_invoice_id: None,
      matched_received_invoice_id: None,
      receipt_path: None,
    }
  }

  pub fn is_matched(&self) -> bool {
    self.matched_invoice_id.is_some()
      || self.matched_received_invoice_id.is_some()
      || self.receipt_path.is_some()
  }
}

/// Uploaded vendor bill PDF with minimal metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedInvoice {
  pub id: Uuid,
  pub company_id: Uuid,
  pub vendor_name: String,
  pub amount: Decimal,
  pub currency: String,
  pub invoice_date: Option<NaiveDate>,
  pub invoice_number: Option<String>,
  pub pdf_path: String,
  pub pdf_drive_file_id: Option<String>,
  pub notes: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl ReceivedInvoice {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    company_id: Uuid,
    vendor_name: String,
    amount: Decimal,
    currency: String,
    invoice_date: Option<NaiveDate>,
    invoice_number: Option<String>,
    pdf_path: String,
    notes: Option<String>,
  ) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4(),
      company_id,
      vendor_name,
      amount,
      currency,
      invoice_date,
      invoice_number,
      pdf_path,
      pdf_drive_file_id: None,
      notes,
      created_at: now,
      updated_at: now,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rust_decimal_macros::dec;

  #[test]
  fn test_monthly_report_new_defaults() {
    let company_id = Uuid::new_v4();
    let report = MonthlyReport::new(company_id, 3, 2026, "EE123456789".to_string());

    assert_eq!(report.company_id, company_id);
    assert_eq!(report.month, 3);
    assert_eq!(report.year, 2026);
    assert_eq!(report.status, ReportStatus::Draft);
    assert_eq!(report.bank_account_iban, "EE123456789");
    assert_eq!(report.total_incoming, Decimal::ZERO);
    assert_eq!(report.total_outgoing, Decimal::ZERO);
    assert_eq!(report.transaction_count, 0);
    assert_eq!(report.matched_count, 0);
    assert!(report.drive_folder_id.is_none());
  }

  #[test]
  fn test_bank_transaction_new_defaults() {
    let report_id = Uuid::new_v4();
    let date = NaiveDate::from_ymd_opt(2026, 3, 15).unwrap();
    let tx = BankTransaction::new(
      report_id,
      1,
      date,
      Some("Acme Corp".to_string()),
      Some("EE987654321".to_string()),
      TransactionDirection::Debit,
      dec!(1300.00),
      None,
      Some("Payment".to_string()),
      "EUR".to_string(),
      None,
    );

    assert_eq!(tx.report_id, report_id);
    assert_eq!(tx.row_number, 1);
    assert_eq!(tx.date, date);
    assert_eq!(tx.direction, TransactionDirection::Debit);
    assert_eq!(tx.amount, dec!(1300.00));
    assert!(tx.matched_invoice_id.is_none());
    assert!(tx.matched_received_invoice_id.is_none());
  }

  #[test]
  fn test_bank_transaction_is_matched_false() {
    let tx = BankTransaction::new(
      Uuid::new_v4(),
      1,
      NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
      None,
      None,
      TransactionDirection::Credit,
      dec!(500.00),
      None,
      None,
      "EUR".to_string(),
      None,
    );
    assert!(!tx.is_matched());
  }

  #[test]
  fn test_bank_transaction_is_matched_with_invoice() {
    let mut tx = BankTransaction::new(
      Uuid::new_v4(),
      1,
      NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
      None,
      None,
      TransactionDirection::Credit,
      dec!(500.00),
      None,
      None,
      "EUR".to_string(),
      None,
    );
    tx.matched_invoice_id = Some(Uuid::new_v4());
    assert!(tx.is_matched());
  }

  #[test]
  fn test_bank_transaction_is_matched_with_received_invoice() {
    let mut tx = BankTransaction::new(
      Uuid::new_v4(),
      1,
      NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
      None,
      None,
      TransactionDirection::Debit,
      dec!(500.00),
      None,
      None,
      "EUR".to_string(),
      None,
    );
    tx.matched_received_invoice_id = Some(Uuid::new_v4());
    assert!(tx.is_matched());
  }

  #[test]
  fn test_received_invoice_new_defaults() {
    let company_id = Uuid::new_v4();
    let inv = ReceivedInvoice::new(
      company_id,
      "Vendor Co".to_string(),
      dec!(2500.00),
      "EUR".to_string(),
      Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()),
      Some("INV-001".to_string()),
      "/uploads/invoice.pdf".to_string(),
      None,
    );

    assert_eq!(inv.company_id, company_id);
    assert_eq!(inv.vendor_name, "Vendor Co");
    assert_eq!(inv.amount, dec!(2500.00));
    assert!(inv.pdf_drive_file_id.is_none());
  }
}

/// Parsed transaction from CSV (before being persisted)
#[derive(Debug, Clone)]
pub struct ParsedTransaction {
  pub row_number: i32,
  pub client_account: String,
  pub date: NaiveDate,
  pub counterparty_name: Option<String>,
  pub counterparty_account: Option<String>,
  pub direction: TransactionDirection,
  pub amount: Decimal,
  pub reference_number: Option<String>,
  pub description: Option<String>,
  pub currency: String,
  pub registry_code: Option<String>,
}
