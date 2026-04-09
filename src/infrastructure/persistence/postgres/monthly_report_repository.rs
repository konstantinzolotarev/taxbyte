use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::report::{MonthlyReport, MonthlyReportRepository, ReportError, ReportStatus};

#[derive(Debug, FromRow)]
struct MonthlyReportRow {
  id: Uuid,
  company_id: Uuid,
  month: i32,
  year: i32,
  status: String,
  bank_account_iban: Option<String>,
  total_incoming: Decimal,
  total_outgoing: Decimal,
  transaction_count: i32,
  matched_count: i32,
  drive_folder_id: Option<String>,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
}

impl TryFrom<MonthlyReportRow> for MonthlyReport {
  type Error = ReportError;

  fn try_from(row: MonthlyReportRow) -> Result<Self, Self::Error> {
    Ok(MonthlyReport {
      id: row.id,
      company_id: row.company_id,
      month: row.month as u32,
      year: row.year,
      status: ReportStatus::try_from(row.status.as_str())?,
      bank_account_iban: row.bank_account_iban,
      total_incoming: row.total_incoming,
      total_outgoing: row.total_outgoing,
      transaction_count: row.transaction_count,
      matched_count: row.matched_count,
      drive_folder_id: row.drive_folder_id,
      created_at: row.created_at,
      updated_at: row.updated_at,
    })
  }
}

pub struct PostgresMonthlyReportRepository {
  pool: PgPool,
}

impl PostgresMonthlyReportRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl MonthlyReportRepository for PostgresMonthlyReportRepository {
  async fn create(&self, report: MonthlyReport) -> Result<MonthlyReport, ReportError> {
    let row = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            INSERT INTO monthly_reports (id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            "#,
        )
        .bind(report.id)
        .bind(report.company_id)
        .bind(report.month as i32)
        .bind(report.year)
        .bind(report.status.as_str())
        .bind(report.bank_account_iban.as_deref())
        .bind(report.total_incoming)
        .bind(report.total_outgoing)
        .bind(report.transaction_count)
        .bind(report.matched_count)
        .bind(report.drive_folder_id.as_deref())
        .bind(report.created_at)
        .bind(report.updated_at)
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<MonthlyReport>, ReportError> {
    let row = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            SELECT id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            FROM monthly_reports WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn find_by_company_and_period(
    &self,
    company_id: Uuid,
    month: u32,
    year: i32,
  ) -> Result<Option<MonthlyReport>, ReportError> {
    let row = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            SELECT id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            FROM monthly_reports WHERE company_id = $1 AND month = $2 AND year = $3
            "#,
        )
        .bind(company_id)
        .bind(month as i32)
        .bind(year)
        .fetch_optional(&self.pool)
        .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<MonthlyReport>, ReportError> {
    let rows = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            SELECT id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            FROM monthly_reports WHERE company_id = $1 ORDER BY year DESC, month DESC
            "#,
        )
        .bind(company_id)
        .fetch_all(&self.pool)
        .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn update(&self, report: MonthlyReport) -> Result<MonthlyReport, ReportError> {
    let row = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            UPDATE monthly_reports
            SET status = $2, bank_account_iban = $3, total_incoming = $4, total_outgoing = $5, transaction_count = $6, matched_count = $7, drive_folder_id = $8, updated_at = $9
            WHERE id = $1
            RETURNING id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            "#,
        )
        .bind(report.id)
        .bind(report.status.as_str())
        .bind(report.bank_account_iban.as_deref())
        .bind(report.total_incoming)
        .bind(report.total_outgoing)
        .bind(report.transaction_count)
        .bind(report.matched_count)
        .bind(report.drive_folder_id.as_deref())
        .bind(report.updated_at)
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn delete(&self, id: Uuid) -> Result<(), ReportError> {
    sqlx::query("DELETE FROM monthly_reports WHERE id = $1")
      .bind(id)
      .execute(&self.pool)
      .await?;
    Ok(())
  }
}
