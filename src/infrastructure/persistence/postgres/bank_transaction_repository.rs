use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::report::{
  BankTransaction, BankTransactionRepository, ReportError, TransactionDirection,
};

#[derive(Debug, FromRow)]
struct BankTransactionRow {
  id: Uuid,
  report_id: Uuid,
  row_number: i32,
  date: NaiveDate,
  counterparty_name: Option<String>,
  counterparty_account: Option<String>,
  direction: String,
  amount: Decimal,
  reference_number: Option<String>,
  description: Option<String>,
  currency: String,
  registry_code: Option<String>,
  matched_invoice_id: Option<Uuid>,
  matched_received_invoice_id: Option<Uuid>,
}

impl TryFrom<BankTransactionRow> for BankTransaction {
  type Error = ReportError;

  fn try_from(row: BankTransactionRow) -> Result<Self, Self::Error> {
    Ok(BankTransaction {
      id: row.id,
      report_id: row.report_id,
      row_number: row.row_number,
      date: row.date,
      counterparty_name: row.counterparty_name,
      counterparty_account: row.counterparty_account,
      direction: TransactionDirection::try_from(row.direction.as_str())?,
      amount: row.amount,
      reference_number: row.reference_number,
      description: row.description,
      currency: row.currency,
      registry_code: row.registry_code,
      matched_invoice_id: row.matched_invoice_id,
      matched_received_invoice_id: row.matched_received_invoice_id,
    })
  }
}

pub struct PostgresBankTransactionRepository {
  pool: PgPool,
}

impl PostgresBankTransactionRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl BankTransactionRepository for PostgresBankTransactionRepository {
  async fn create_many(
    &self,
    transactions: Vec<BankTransaction>,
  ) -> Result<Vec<BankTransaction>, ReportError> {
    let now = chrono::Utc::now();
    for tx in &transactions {
      sqlx::query(
                r#"
                INSERT INTO bank_transactions (id, report_id, row_number, date, counterparty_name, counterparty_account, direction, amount, reference_number, description, currency, registry_code, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                "#,
            )
            .bind(tx.id)
            .bind(tx.report_id)
            .bind(tx.row_number)
            .bind(tx.date)
            .bind(tx.counterparty_name.as_deref())
            .bind(tx.counterparty_account.as_deref())
            .bind(tx.direction.as_str())
            .bind(tx.amount)
            .bind(tx.reference_number.as_deref())
            .bind(tx.description.as_deref())
            .bind(&tx.currency)
            .bind(tx.registry_code.as_deref())
            .bind(now)
            .execute(&self.pool)
            .await?;
    }
    Ok(transactions)
  }

  async fn find_by_report_id(&self, report_id: Uuid) -> Result<Vec<BankTransaction>, ReportError> {
    let rows = sqlx::query_as::<_, BankTransactionRow>(
            r#"
            SELECT id, report_id, row_number, date, counterparty_name, counterparty_account, direction, amount, reference_number, description, currency, registry_code, matched_invoice_id, matched_received_invoice_id
            FROM bank_transactions WHERE report_id = $1 ORDER BY row_number
            "#,
        )
        .bind(report_id)
        .fetch_all(&self.pool)
        .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<BankTransaction>, ReportError> {
    let row = sqlx::query_as::<_, BankTransactionRow>(
            r#"
            SELECT id, report_id, row_number, date, counterparty_name, counterparty_account, direction, amount, reference_number, description, currency, registry_code, matched_invoice_id, matched_received_invoice_id
            FROM bank_transactions WHERE id = $1
            "#,
        )
        .bind(id)
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
            SET matched_invoice_id = $2, matched_received_invoice_id = $3
            WHERE id = $1
            "#,
    )
    .bind(transaction_id)
    .bind(invoice_id)
    .bind(received_invoice_id)
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  async fn clear_match(&self, transaction_id: Uuid) -> Result<(), ReportError> {
    sqlx::query(
      r#"
            UPDATE bank_transactions
            SET matched_invoice_id = NULL, matched_received_invoice_id = NULL
            WHERE id = $1
            "#,
    )
    .bind(transaction_id)
    .execute(&self.pool)
    .await?;
    Ok(())
  }

  async fn delete_by_report_id(&self, report_id: Uuid) -> Result<(), ReportError> {
    sqlx::query("DELETE FROM bank_transactions WHERE report_id = $1")
      .bind(report_id)
      .execute(&self.pool)
      .await?;
    Ok(())
  }
}
