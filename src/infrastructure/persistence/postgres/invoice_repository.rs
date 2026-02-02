use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  Currency, Invoice, InvoiceNumber, InvoiceStatus, PaymentTerms, errors::InvoiceError,
  ports::InvoiceRepository,
};

#[derive(Debug, FromRow)]
struct InvoiceRow {
  id: Uuid,
  company_id: Uuid,
  customer_id: Uuid,
  bank_account_id: Option<Uuid>,
  invoice_number: String,
  invoice_date: NaiveDate,
  due_date: NaiveDate,
  payment_terms: String,
  currency: String,
  status: String,
  pdf_path: Option<String>,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
  archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<InvoiceRow> for Invoice {
  type Error = InvoiceError;

  fn try_from(row: InvoiceRow) -> Result<Self, Self::Error> {
    let invoice_number = InvoiceNumber::new(row.invoice_number)?;
    let payment_terms = PaymentTerms::from_str(&row.payment_terms)?;
    let currency = Currency::from_str(&row.currency)?;
    let status = InvoiceStatus::from_str(&row.status)?;

    Ok(Invoice {
      id: row.id,
      company_id: row.company_id,
      customer_id: row.customer_id,
      bank_account_id: row.bank_account_id,
      invoice_number,
      invoice_date: row.invoice_date,
      due_date: row.due_date,
      payment_terms,
      currency,
      status,
      pdf_path: row.pdf_path,
      created_at: row.created_at,
      updated_at: row.updated_at,
      archived_at: row.archived_at,
    })
  }
}

pub struct PostgresInvoiceRepository {
  pool: PgPool,
}

impl PostgresInvoiceRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceRepository for PostgresInvoiceRepository {
  async fn create(&self, invoice: Invoice) -> Result<Invoice, InvoiceError> {
    let invoice_number_value = invoice.invoice_number.value().to_string();

    let row = sqlx::query_as::<_, InvoiceRow>(
      r#"
            INSERT INTO invoices (
                id, company_id, customer_id, bank_account_id, invoice_number,
                invoice_date, due_date, payment_terms, currency, status,
                pdf_path, created_at, updated_at, archived_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, company_id, customer_id, bank_account_id, invoice_number,
                      invoice_date, due_date, payment_terms, currency, status,
                      pdf_path, created_at, updated_at, archived_at
            "#,
    )
    .bind(invoice.id)
    .bind(invoice.company_id)
    .bind(invoice.customer_id)
    .bind(invoice.bank_account_id)
    .bind(invoice.invoice_number.value())
    .bind(invoice.invoice_date)
    .bind(invoice.due_date)
    .bind(invoice.payment_terms.as_str())
    .bind(invoice.currency.as_str())
    .bind(invoice.status.as_str())
    .bind(invoice.pdf_path)
    .bind(invoice.created_at)
    .bind(invoice.updated_at)
    .bind(invoice.archived_at)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      if let sqlx::Error::Database(db_err) = &e {
        // Check for unique constraint violation
        if db_err.code().as_deref() == Some("23505") {
          // PostgreSQL unique violation code
          if db_err.constraint() == Some("invoices_company_number_unique") {
            return InvoiceError::InvoiceNumberAlreadyExists(invoice_number_value);
          }
        }
      }
      InvoiceError::Database(e)
    })?;

    row.try_into()
  }

  async fn update(&self, invoice: Invoice) -> Result<Invoice, InvoiceError> {
    let row = sqlx::query_as::<_, InvoiceRow>(
      r#"
            UPDATE invoices
            SET customer_id = $2, bank_account_id = $3, invoice_date = $4,
                due_date = $5, payment_terms = $6, status = $7,
                pdf_path = $8, updated_at = $9, archived_at = $10
            WHERE id = $1
            RETURNING id, company_id, customer_id, bank_account_id, invoice_number,
                      invoice_date, due_date, payment_terms, currency, status,
                      pdf_path, created_at, updated_at, archived_at
            "#,
    )
    .bind(invoice.id)
    .bind(invoice.customer_id)
    .bind(invoice.bank_account_id)
    .bind(invoice.invoice_date)
    .bind(invoice.due_date)
    .bind(invoice.payment_terms.as_str())
    .bind(invoice.status.as_str())
    .bind(invoice.pdf_path)
    .bind(invoice.updated_at)
    .bind(invoice.archived_at)
    .fetch_one(&self.pool)
    .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError> {
    let row = sqlx::query_as::<_, InvoiceRow>(
      r#"
            SELECT id, company_id, customer_id, bank_account_id, invoice_number,
                   invoice_date, due_date, payment_terms, currency, status,
                   pdf_path, created_at, updated_at, archived_at
            FROM invoices
            WHERE id = $1
            "#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<Invoice>, InvoiceError> {
    let rows = sqlx::query_as::<_, InvoiceRow>(
      r#"
            SELECT id, company_id, customer_id, bank_account_id, invoice_number,
                   invoice_date, due_date, payment_terms, currency, status,
                   pdf_path, created_at, updated_at, archived_at
            FROM invoices
            WHERE company_id = $1 AND archived_at IS NULL
            ORDER BY invoice_number DESC
            "#,
    )
    .bind(company_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_by_company_and_status(
    &self,
    company_id: Uuid,
    status: InvoiceStatus,
  ) -> Result<Vec<Invoice>, InvoiceError> {
    let rows = sqlx::query_as::<_, InvoiceRow>(
      r#"
            SELECT id, company_id, customer_id, bank_account_id, invoice_number,
                   invoice_date, due_date, payment_terms, currency, status,
                   pdf_path, created_at, updated_at, archived_at
            FROM invoices
            WHERE company_id = $1 AND status = $2 AND archived_at IS NULL
            ORDER BY invoice_number DESC
            "#,
    )
    .bind(company_id)
    .bind(status.as_str())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_by_company_and_customer(
    &self,
    company_id: Uuid,
    customer_id: Uuid,
  ) -> Result<Vec<Invoice>, InvoiceError> {
    let rows = sqlx::query_as::<_, InvoiceRow>(
      r#"
            SELECT id, company_id, customer_id, bank_account_id, invoice_number,
                   invoice_date, due_date, payment_terms, currency, status,
                   pdf_path, created_at, updated_at, archived_at
            FROM invoices
            WHERE company_id = $1 AND customer_id = $2 AND archived_at IS NULL
            ORDER BY invoice_number DESC
            "#,
    )
    .bind(company_id)
    .bind(customer_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_overdue(
    &self,
    company_id: Uuid,
    current_date: NaiveDate,
  ) -> Result<Vec<Invoice>, InvoiceError> {
    let rows = sqlx::query_as::<_, InvoiceRow>(
      r#"
            SELECT id, company_id, customer_id, bank_account_id, invoice_number,
                   invoice_date, due_date, payment_terms, currency, status,
                   pdf_path, created_at, updated_at, archived_at
            FROM invoices
            WHERE company_id = $1 AND status = 'sent' AND due_date < $2 AND archived_at IS NULL
            ORDER BY due_date ASC
            "#,
    )
    .bind(company_id)
    .bind(current_date)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn delete(&self, id: Uuid) -> Result<(), InvoiceError> {
    // First delete all line items
    sqlx::query(
      r#"
      DELETE FROM invoice_line_items
      WHERE invoice_id = $1
      "#,
    )
    .bind(id)
    .execute(&self.pool)
    .await?;

    // Then delete the invoice
    sqlx::query(
      r#"
      DELETE FROM invoices
      WHERE id = $1
      "#,
    )
    .bind(id)
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
