use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::report::{ReceivedInvoice, ReceivedInvoiceRepository, ReportError};

#[derive(Debug, FromRow)]
struct ReceivedInvoiceRow {
  id: Uuid,
  company_id: Uuid,
  vendor_name: String,
  amount: Decimal,
  currency: String,
  invoice_date: Option<NaiveDate>,
  invoice_number: Option<String>,
  pdf_path: String,
  pdf_drive_file_id: Option<String>,
  notes: Option<String>,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
}

impl From<ReceivedInvoiceRow> for ReceivedInvoice {
  fn from(row: ReceivedInvoiceRow) -> Self {
    ReceivedInvoice {
      id: row.id,
      company_id: row.company_id,
      vendor_name: row.vendor_name,
      amount: row.amount,
      currency: row.currency,
      invoice_date: row.invoice_date,
      invoice_number: row.invoice_number,
      pdf_path: row.pdf_path,
      pdf_drive_file_id: row.pdf_drive_file_id,
      notes: row.notes,
      created_at: row.created_at,
      updated_at: row.updated_at,
    }
  }
}

pub struct PostgresReceivedInvoiceRepository {
  pool: PgPool,
}

impl PostgresReceivedInvoiceRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl ReceivedInvoiceRepository for PostgresReceivedInvoiceRepository {
  async fn create(&self, invoice: ReceivedInvoice) -> Result<ReceivedInvoice, ReportError> {
    let row = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            INSERT INTO received_invoices (id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            "#,
        )
        .bind(invoice.id)
        .bind(invoice.company_id)
        .bind(&invoice.vendor_name)
        .bind(invoice.amount)
        .bind(&invoice.currency)
        .bind(invoice.invoice_date)
        .bind(invoice.invoice_number.as_deref())
        .bind(&invoice.pdf_path)
        .bind(invoice.pdf_drive_file_id.as_deref())
        .bind(invoice.notes.as_deref())
        .bind(invoice.created_at)
        .bind(invoice.updated_at)
        .fetch_one(&self.pool)
        .await?;

    Ok(row.into())
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<ReceivedInvoice>, ReportError> {
    let row = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            SELECT id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            FROM received_invoices WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

    Ok(row.map(|r| r.into()))
  }

  async fn find_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<ReceivedInvoice>, ReportError> {
    let rows = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            SELECT id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            FROM received_invoices WHERE company_id = $1 ORDER BY created_at DESC
            "#,
        )
        .bind(company_id)
        .fetch_all(&self.pool)
        .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
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
            WHERE company_id = $1 AND invoice_date >= $2 AND invoice_date <= $3
            ORDER BY invoice_date
            "#,
        )
        .bind(company_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
  }

  async fn find_unmatched_by_company(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<ReceivedInvoice>, ReportError> {
    let rows = sqlx::query_as::<_, ReceivedInvoiceRow>(
            r#"
            SELECT id, company_id, vendor_name, amount, currency, invoice_date, invoice_number, pdf_path, pdf_drive_file_id, notes, created_at, updated_at
            FROM received_invoices
            WHERE company_id = $1
              AND id NOT IN (SELECT matched_received_invoice_id FROM bank_transactions WHERE matched_received_invoice_id IS NOT NULL)
            ORDER BY created_at DESC
            "#,
        )
        .bind(company_id)
        .fetch_all(&self.pool)
        .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
  }

  async fn delete(&self, id: Uuid) -> Result<(), ReportError> {
    sqlx::query("DELETE FROM received_invoices WHERE id = $1")
      .bind(id)
      .execute(&self.pool)
      .await?;
    Ok(())
  }
}
