use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::report::{
  BankTransaction, BankTransactionRepository, ReportError, TransactionDirection,
};

#[derive(Debug, FromRow)]
struct BankTransactionRow {
  id: String,
  report_id: String,
  row_number: i32,
  date: String,
  counterparty_name: Option<String>,
  counterparty_account: Option<String>,
  direction: String,
  amount: String,
  reference_number: Option<String>,
  description: Option<String>,
  currency: String,
  registry_code: Option<String>,
  matched_invoice_id: Option<String>,
  matched_received_invoice_id: Option<String>,
  receipt_path: Option<String>,
}

impl TryFrom<BankTransactionRow> for BankTransaction {
  type Error = ReportError;

  fn try_from(row: BankTransactionRow) -> Result<Self, Self::Error> {
    Ok(BankTransaction {
      id: Uuid::parse_str(&row.id)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      report_id: Uuid::parse_str(&row.report_id)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      row_number: row.row_number,
      date: NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      counterparty_name: row.counterparty_name,
      counterparty_account: row.counterparty_account,
      direction: TransactionDirection::try_from(row.direction.as_str())?,
      amount: Decimal::from_str(&row.amount)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      reference_number: row.reference_number,
      description: row.description,
      currency: row.currency,
      registry_code: row.registry_code,
      matched_invoice_id: row
        .matched_invoice_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      matched_received_invoice_id: row
        .matched_received_invoice_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      receipt_path: row.receipt_path,
    })
  }
}

pub struct SqliteBankTransactionRepository {
  pool: SqlitePool,
}

impl SqliteBankTransactionRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl BankTransactionRepository for SqliteBankTransactionRepository {
  async fn create_many(
    &self,
    transactions: Vec<BankTransaction>,
  ) -> Result<Vec<BankTransaction>, ReportError> {
    let now = Utc::now().to_rfc3339();
    for tx in &transactions {
      sqlx::query(
                r#"
                INSERT INTO bank_transactions (id, report_id, row_number, date, counterparty_name, counterparty_account, direction, amount, reference_number, description, currency, registry_code, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
            )
            .bind(tx.id.to_string())
            .bind(tx.report_id.to_string())
            .bind(tx.row_number)
            .bind(tx.date.format("%Y-%m-%d").to_string())
            .bind(tx.counterparty_name.as_deref())
            .bind(tx.counterparty_account.as_deref())
            .bind(tx.direction.as_str())
            .bind(tx.amount.to_string())
            .bind(tx.reference_number.as_deref())
            .bind(tx.description.as_deref())
            .bind(&tx.currency)
            .bind(tx.registry_code.as_deref())
            .bind(&now)
            .execute(&self.pool)
            .await?;
    }
    Ok(transactions)
  }

  async fn find_by_report_id(&self, report_id: Uuid) -> Result<Vec<BankTransaction>, ReportError> {
    let rows = sqlx::query_as::<_, BankTransactionRow>(
            r#"
            SELECT id, report_id, row_number, date, counterparty_name, counterparty_account, direction, amount, reference_number, description, currency, registry_code, matched_invoice_id, matched_received_invoice_id, receipt_path
            FROM bank_transactions WHERE report_id = ?1 ORDER BY row_number
            "#,
        )
        .bind(report_id.to_string())
        .fetch_all(&self.pool)
        .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<BankTransaction>, ReportError> {
    let row = sqlx::query_as::<_, BankTransactionRow>(
            r#"
            SELECT id, report_id, row_number, date, counterparty_name, counterparty_account, direction, amount, reference_number, description, currency, registry_code, matched_invoice_id, matched_received_invoice_id, receipt_path
            FROM bank_transactions WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn update_match(
    &self,
    transaction_id: Uuid,
    invoice_id: Option<Uuid>,
    received_invoice_id: Option<Uuid>,
  ) -> Result<(), ReportError> {
    sqlx::query(
      r#"
            UPDATE bank_transactions
            SET matched_invoice_id = ?2, matched_received_invoice_id = ?3
            WHERE id = ?1
            "#,
    )
    .bind(transaction_id.to_string())
    .bind(invoice_id.map(|id| id.to_string()))
    .bind(received_invoice_id.map(|id| id.to_string()))
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  async fn clear_match(&self, transaction_id: Uuid) -> Result<(), ReportError> {
    sqlx::query(
      r#"
            UPDATE bank_transactions
            SET matched_invoice_id = NULL, matched_received_invoice_id = NULL
            WHERE id = ?1
            "#,
    )
    .bind(transaction_id.to_string())
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  async fn update_receipt_path(
    &self,
    transaction_id: Uuid,
    receipt_path: Option<String>,
  ) -> Result<(), ReportError> {
    sqlx::query(
      r#"
            UPDATE bank_transactions
            SET receipt_path = ?2
            WHERE id = ?1
            "#,
    )
    .bind(transaction_id.to_string())
    .bind(receipt_path)
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  async fn delete_by_report_id(&self, report_id: Uuid) -> Result<(), ReportError> {
    sqlx::query("DELETE FROM bank_transactions WHERE report_id = ?1")
      .bind(report_id.to_string())
      .execute(&self.pool)
      .await?;
    Ok(())
  }
}
