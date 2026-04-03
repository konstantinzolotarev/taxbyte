use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  Currency, Invoice, InvoiceNumber, InvoiceStatus, PaymentTerms, errors::InvoiceError,
  ports::InvoiceRepository,
};

#[derive(Debug, FromRow)]
struct InvoiceRow {
  id: String,
  company_id: String,
  customer_id: String,
  bank_account_id: Option<String>,
  invoice_number: String,
  invoice_date: String,
  due_date: String,
  payment_terms: String,
  currency: String,
  status: String,
  pdf_path: Option<String>,
  pdf_drive_file_id: Option<String>,
  created_at: String,
  updated_at: String,
  archived_at: Option<String>,
}

fn parse_invoice_row(row: InvoiceRow) -> Result<Invoice, InvoiceError> {
  let id = Uuid::parse_str(&row.id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let company_id = Uuid::parse_str(&row.company_id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let customer_id = Uuid::parse_str(&row.customer_id)
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;
  let bank_account_id = row
    .bank_account_id
    .map(|s| Uuid::parse_str(&s))
    .transpose()
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse UUID: {}", e)))?;

  let invoice_number = InvoiceNumber::new(row.invoice_number)?;
  let invoice_date = NaiveDate::parse_from_str(&row.invoice_date, "%Y-%m-%d")
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse date: {}", e)))?;
  let due_date = NaiveDate::parse_from_str(&row.due_date, "%Y-%m-%d")
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse date: {}", e)))?;
  let payment_terms = PaymentTerms::from_str(&row.payment_terms)?;
  let currency = Currency::from_str(&row.currency)?;
  let status = InvoiceStatus::from_str(&row.status)?;

  let created_at = DateTime::parse_from_rfc3339(&row.created_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse datetime: {}", e)))?;
  let updated_at = DateTime::parse_from_rfc3339(&row.updated_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| InvoiceError::Internal(format!("Failed to parse datetime: {}", e)))?;
  let archived_at = row
    .archived_at
    .map(|s| {
      DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| InvoiceError::Internal(format!("Failed to parse datetime: {}", e)))
    })
    .transpose()?;

  Ok(Invoice {
    id,
    company_id,
    customer_id,
    bank_account_id,
    invoice_number,
    invoice_date,
    due_date,
    payment_terms,
    currency,
    status,
    pdf_path: row.pdf_path,
    pdf_drive_file_id: row.pdf_drive_file_id,
    created_at,
    updated_at,
    archived_at,
  })
}

pub struct SqliteInvoiceRepository {
  pool: SqlitePool,
}

impl SqliteInvoiceRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceRepository for SqliteInvoiceRepository {
  async fn create(&self, invoice: Invoice) -> Result<Invoice, InvoiceError> {
    let invoice_number_value = invoice.invoice_number.value().to_string();

    let row = sqlx::query_as::<_, InvoiceRow>(
      r#"
      INSERT INTO invoices (
          id, company_id, customer_id, bank_account_id, invoice_number,
          invoice_date, due_date, payment_terms, currency, status,
          pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      )
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
      RETURNING id, company_id, customer_id, bank_account_id, invoice_number,
                invoice_date, due_date, payment_terms, currency, status,
                pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      "#,
    )
    .bind(invoice.id.to_string())
    .bind(invoice.company_id.to_string())
    .bind(invoice.customer_id.to_string())
    .bind(invoice.bank_account_id.map(|id| id.to_string()))
    .bind(invoice.invoice_number.value())
    .bind(invoice.invoice_date.format("%Y-%m-%d").to_string())
    .bind(invoice.due_date.format("%Y-%m-%d").to_string())
    .bind(invoice.payment_terms.as_str())
    .bind(invoice.currency.as_str())
    .bind(invoice.status.as_str())
    .bind(invoice.pdf_path)
    .bind(invoice.pdf_drive_file_id)
    .bind(invoice.created_at.to_rfc3339())
    .bind(invoice.updated_at.to_rfc3339())
    .bind(invoice.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      if let sqlx::Error::Database(db_err) = &e {
        if db_err.is_unique_violation() {
          return InvoiceError::InvoiceNumberAlreadyExists(invoice_number_value);
        }
      }
      InvoiceError::Database(e)
    })?;

    parse_invoice_row(row)
  }

  async fn update(&self, invoice: Invoice) -> Result<Invoice, InvoiceError> {
    let row = sqlx::query_as::<_, InvoiceRow>(
      r#"
      UPDATE invoices
      SET customer_id = ?2, bank_account_id = ?3, invoice_date = ?4,
          due_date = ?5, payment_terms = ?6, status = ?7,
          pdf_path = ?8, pdf_drive_file_id = ?9, updated_at = ?10, archived_at = ?11
      WHERE id = ?1
      RETURNING id, company_id, customer_id, bank_account_id, invoice_number,
                invoice_date, due_date, payment_terms, currency, status,
                pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      "#,
    )
    .bind(invoice.id.to_string())
    .bind(invoice.customer_id.to_string())
    .bind(invoice.bank_account_id.map(|id| id.to_string()))
    .bind(invoice.invoice_date.format("%Y-%m-%d").to_string())
    .bind(invoice.due_date.format("%Y-%m-%d").to_string())
    .bind(invoice.payment_terms.as_str())
    .bind(invoice.status.as_str())
    .bind(invoice.pdf_path)
    .bind(invoice.pdf_drive_file_id)
    .bind(invoice.updated_at.to_rfc3339())
    .bind(invoice.archived_at.map(|dt| dt.to_rfc3339()))
    .fetch_one(&self.pool)
    .await?;

    parse_invoice_row(row)
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError> {
    let row = sqlx::query_as::<_, InvoiceRow>(
      r#"
      SELECT id, company_id, customer_id, bank_account_id, invoice_number,
             invoice_date, due_date, payment_terms, currency, status,
             pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      FROM invoices
      WHERE id = ?1
      "#,
    )
    .bind(id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    row.map(parse_invoice_row).transpose()
  }

  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<Invoice>, InvoiceError> {
    let rows = sqlx::query_as::<_, InvoiceRow>(
      r#"
      SELECT id, company_id, customer_id, bank_account_id, invoice_number,
             invoice_date, due_date, payment_terms, currency, status,
             pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      FROM invoices
      WHERE company_id = ?1 AND archived_at IS NULL
      ORDER BY invoice_number DESC
      "#,
    )
    .bind(company_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_invoice_row).collect()
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
             pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      FROM invoices
      WHERE company_id = ?1 AND status = ?2 AND archived_at IS NULL
      ORDER BY invoice_number DESC
      "#,
    )
    .bind(company_id.to_string())
    .bind(status.as_str())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_invoice_row).collect()
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
             pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      FROM invoices
      WHERE company_id = ?1 AND customer_id = ?2 AND archived_at IS NULL
      ORDER BY invoice_number DESC
      "#,
    )
    .bind(company_id.to_string())
    .bind(customer_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_invoice_row).collect()
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
             pdf_path, pdf_drive_file_id, created_at, updated_at, archived_at
      FROM invoices
      WHERE company_id = ?1 AND status = 'sent' AND due_date < ?2 AND archived_at IS NULL
      ORDER BY due_date ASC
      "#,
    )
    .bind(company_id.to_string())
    .bind(current_date.format("%Y-%m-%d").to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_invoice_row).collect()
  }

  async fn delete(&self, id: Uuid) -> Result<(), InvoiceError> {
    // First delete all line items
    sqlx::query("DELETE FROM invoice_line_items WHERE invoice_id = ?1")
      .bind(id.to_string())
      .execute(&self.pool)
      .await?;

    // Then delete the invoice
    sqlx::query("DELETE FROM invoices WHERE id = ?1")
      .bind(id.to_string())
      .execute(&self.pool)
      .await?;

    Ok(())
  }
}
