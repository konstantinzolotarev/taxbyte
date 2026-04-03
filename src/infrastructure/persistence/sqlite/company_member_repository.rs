use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::domain::auth::errors::RepositoryError;
use crate::domain::company::{CompanyError, CompanyMember, CompanyMemberRepository};

#[derive(Debug, FromRow)]
struct CompanyMemberRow {
  company_id: String,
  user_id: String,
  role: String,
  created_at: String,
}

fn parse_member_row(row: CompanyMemberRow) -> Result<CompanyMember, CompanyError> {
  let company_id = Uuid::parse_str(&row.company_id)
    .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let user_id = Uuid::parse_str(&row.user_id)
    .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  let created_at = DateTime::parse_from_rfc3339(&row.created_at)
    .map(|dt| dt.with_timezone(&Utc))
    .map_err(|e| CompanyError::Repository(RepositoryError::QueryFailed(e.to_string())))?;
  CompanyMember::from_db(company_id, user_id, row.role, created_at)
}

pub struct SqliteCompanyMemberRepository {
  pool: SqlitePool,
}

impl SqliteCompanyMemberRepository {
  pub fn new(pool: SqlitePool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl CompanyMemberRepository for SqliteCompanyMemberRepository {
  async fn add_member(&self, member: CompanyMember) -> Result<CompanyMember, CompanyError> {
    let row = sqlx::query_as::<_, CompanyMemberRow>(
      r#"
      INSERT INTO company_members (id, company_id, user_id, role, created_at, updated_at)
      VALUES (?1, ?2, ?3, ?4, ?5, ?5)
      RETURNING company_id, user_id, role, created_at
      "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(member.company_id.to_string())
    .bind(member.user_id.to_string())
    .bind(member.role.as_str())
    .bind(member.joined_at.to_rfc3339())
    .fetch_one(&self.pool)
    .await?;

    parse_member_row(row)
  }

  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<CompanyMember>, CompanyError> {
    let rows = sqlx::query_as::<_, CompanyMemberRow>(
      r#"
      SELECT company_id, user_id, role, created_at
      FROM company_members
      WHERE company_id = ?1
      ORDER BY created_at ASC
      "#,
    )
    .bind(company_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_member_row).collect()
  }

  async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<CompanyMember>, CompanyError> {
    let rows = sqlx::query_as::<_, CompanyMemberRow>(
      r#"
      SELECT company_id, user_id, role, created_at
      FROM company_members
      WHERE user_id = ?1
      ORDER BY created_at DESC
      "#,
    )
    .bind(user_id.to_string())
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(parse_member_row).collect()
  }

  async fn find_member(
    &self,
    company_id: Uuid,
    user_id: Uuid,
  ) -> Result<Option<CompanyMember>, CompanyError> {
    let row = sqlx::query_as::<_, CompanyMemberRow>(
      r#"
      SELECT company_id, user_id, role, created_at
      FROM company_members
      WHERE company_id = ?1 AND user_id = ?2
      "#,
    )
    .bind(company_id.to_string())
    .bind(user_id.to_string())
    .fetch_optional(&self.pool)
    .await?;

    match row {
      Some(r) => Ok(Some(parse_member_row(r)?)),
      None => Ok(None),
    }
  }

  async fn remove_member(&self, company_id: Uuid, user_id: Uuid) -> Result<(), CompanyError> {
    sqlx::query("DELETE FROM company_members WHERE company_id = ?1 AND user_id = ?2")
      .bind(company_id.to_string())
      .bind(user_id.to_string())
      .execute(&self.pool)
      .await?;

    Ok(())
  }
}
