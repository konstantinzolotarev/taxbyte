use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, SqlitePool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::report::{MonthlyReport, MonthlyReportRepository, ReportError, ReportStatus};

#[derive(Debug, FromRow)]
struct MonthlyReportRow {
  id: String,
  company_id: String,
  month: i32,
  year: i32,
  status: String,
  bank_account_iban: String,
  total_incoming: String,
  total_outgoing: String,
  transaction_count: i32,
  matched_count: i32,
  drive_folder_id: Option<String>,
  created_at: String,
  updated_at: String,
}

impl TryFrom<MonthlyReportRow> for MonthlyReport {
  type Error = ReportError;

  fn try_from(row: MonthlyReportRow) -> Result<Self, Self::Error> {
    Ok(MonthlyReport {
      id: Uuid::parse_str(&row.id)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      company_id: Uuid::parse_str(&row.company_id)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      month: row.month as u32,
      year: row.year,
      status: ReportStatus::try_from(row.status.as_str())?,
      bank_account_iban: row.bank_account_iban,
      total_incoming: Decimal::from_str(&row.total_incoming)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      total_outgoing: Decimal::from_str(&row.total_outgoing)
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      transaction_count: row.transaction_count,
      matched_count: row.matched_count,
      drive_folder_id: row.drive_folder_id,
      created_at: DateTime::parse_from_rfc3339(&row.created_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
      updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| ReportError::Repository(RepositoryError::QueryFailed(e.to_string())))?,
    })
  }
}

pub struct SqliteMonthlyReportRepository {
  pool: SqlitePool,
}

impl SqliteMonthlyReportRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl MonthlyReportRepository for SqliteMonthlyReportRepository {
  async fn create(&self, report: MonthlyReport) -> Result<MonthlyReport, ReportError> {
    let row = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            INSERT INTO monthly_reports (id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            RETURNING id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            "#,
        )
        .bind(report.id.to_string())
        .bind(report.company_id.to_string())
        .bind(report.month as i32)
        .bind(report.year)
        .bind(report.status.as_str())
        .bind(&report.bank_account_iban)
        .bind(report.total_incoming.to_string())
        .bind(report.total_outgoing.to_string())
        .bind(report.transaction_count)
        .bind(report.matched_count)
        .bind(report.drive_folder_id.as_deref())
        .bind(report.created_at.to_rfc3339())
        .bind(report.updated_at.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<MonthlyReport>, ReportError> {
    let row = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            SELECT id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            FROM monthly_reports WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
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
            FROM monthly_reports WHERE company_id = ?1 AND month = ?2 AND year = ?3
            "#,
        )
        .bind(company_id.to_string())
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
            FROM monthly_reports WHERE company_id = ?1 ORDER BY year DESC, month DESC
            "#,
        )
        .bind(company_id.to_string())
        .fetch_all(&self.pool)
        .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn update(&self, report: MonthlyReport) -> Result<MonthlyReport, ReportError> {
    let row = sqlx::query_as::<_, MonthlyReportRow>(
            r#"
            UPDATE monthly_reports
            SET status = ?2, total_incoming = ?3, total_outgoing = ?4, transaction_count = ?5, matched_count = ?6, drive_folder_id = ?7, updated_at = ?8
            WHERE id = ?1
            RETURNING id, company_id, month, year, status, bank_account_iban, total_incoming, total_outgoing, transaction_count, matched_count, drive_folder_id, created_at, updated_at
            "#,
        )
        .bind(report.id.to_string())
        .bind(report.status.as_str())
        .bind(report.total_incoming.to_string())
        .bind(report.total_outgoing.to_string())
        .bind(report.transaction_count)
        .bind(report.matched_count)
        .bind(report.drive_folder_id.as_deref())
        .bind(report.updated_at.to_rfc3339())
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn delete(&self, id: Uuid) -> Result<(), ReportError> {
    sqlx::query("DELETE FROM monthly_reports WHERE id = ?1")
      .bind(id.to_string())
      .execute(&self.pool)
      .await?;
    Ok(())
  }
}
