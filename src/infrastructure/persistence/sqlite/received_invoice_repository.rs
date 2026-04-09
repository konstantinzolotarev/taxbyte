use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::report::{ReceivedInvoice, ReceivedInvoiceRepository, ReportError};

#[derive(Debug, FromRow)]
struct ReceivedInvoiceRow {
  id: String,
  company_id: String,
  vendor_name: String,
  amount: String,
  currency: String,
  invoice_date: Option<String>,
  invoice_number: Option<String>,
  pdf_path: String,
  pdf_drive_file_id: Option<String>,
  notes: Option<String>,
  created_at: String,
  updated_at: String,
}

impl TryFrom<ReceivedInvoiceRow> for ReceivedInvoice {
  type Error = ReportError;

  fn try_from(row: ReceivedInvoiceRow) -> Result<Self, Self::Error> {
    Ok(ReceivedInvoice {
      id: Uuid::parse_str(&row.id)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      company_id: Uuid::parse_str(&row.company_id)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      vendor_name: row.vendor_name,
      amount: Decimal::from_str(&row.amount)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      currency: row.currency,
      invoice_date: row
        .invoice_date
        .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .transpose()
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      invoice_number: row.invoice_number,
      pdf_path: row.pdf_path,
      pdf_drive_file_id: row.pdf_drive_file_id,
      notes: row.notes,
      created_at: DateTime::parse_from_rfc3339(&row.created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
    })
  }
}

pub struct SqliteReceivedInvoiceRepository {
  pool: SqlitePool,
}

impl SqliteReceivedInvoiceRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl ReceivedInvoiceRepository for SqliteReceivedInvoiceRepository {
  async fn create(&self, invoice: ReceivedInvoice) -> Result<ReceivedInvoice, ReportError> {
    let row = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            INSERT INTO received_invoices (id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            RETURNING id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            "#,
        )
        .bind(invoice.id.to_string())
        .bind(invoice.company_id.to_string())
        .bind(&invoice.vendor_name)
        .bind(invoice.amount.to_string())
        .bind(&invoice.currency)
        .bind(invoice.invoice_date.map(|d| d.format("%Y-%m-%d").to_string()))
        .bind(invoice.invoice_number.as_deref())
        .bind(&invoice.pdf_path)
        .bind(invoice.pdf_drive_file_id.as_deref())
        .bind(invoice.notes.as_deref())
        .bind(invoice.created_at.to_rfc3339())
        .bind(invoice.updated_at.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<ReceivedInvoice>, ReportError> {
    let row = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            SELECT id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            FROM received_invoices WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn find_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<ReceivedInvoice>, ReportError> {
    let rows = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            SELECT id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            FROM received_invoices WHERE company_id = ?1 ORDER BY created_at DESC
            "#,
        )
        .bind(company_id.to_string())
        .fetch_all(&self.pool)
        .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_by_company_and_date_range(
    &self,
    company_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
  ) -> Result<Vec<ReceivedInvoice>, ReportError> {
    let rows = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            SELECT id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            FROM received_invoices
            WHERE company_id = ?1 AND invoice_date >= ?2 AND invoice_date <= ?3
            ORDER BY invoice_date
            "#,
        )
        .bind(company_id.to_string())
        .bind(start_date.format("%Y-%m-%d").to_string())
        .bind(end_date.format("%Y-%m-%d").to_string())
        .fetch_all(&self.pool)
        .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_unmatched_by_company(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<ReceivedInvoice>, ReportError> {
    let rows = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            SELECT id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            FROM received_invoices
            WHERE company_id = ?1
              AND id NOT IN (SELECT matched_received_invoice_id FROM bank_transactions WHERE matched_received_invoice_id IS NOT NULL)
            ORDER BY created_at DESC
            "#,
        )
        .bind(company_id.to_string())
        .fetch_all(&self.pool)
        .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn delete(&self, id: Uuid) -> Result<(), ReportError> {
    sqlx::query("DELETE FROM received_invoices WHERE id = ?1")
      .bind(id.to_string())
      .execute(&self.pool)
      .await?;
    Ok(())
  }
}
