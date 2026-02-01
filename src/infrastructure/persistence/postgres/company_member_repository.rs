use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyMember, CompanyMemberRepository};

#[derive(Debug, FromRow)]
struct CompanyMemberRow {
  company_id: Uuid,
  user_id: Uuid,
  role: String,
  created_at: DateTime<Utc>,
}

impl TryFrom<CompanyMemberRow> for CompanyMember {
  type Error = CompanyError;

  fn try_from(row: CompanyMemberRow) -> Result<Self, Self::Error> {
    CompanyMember::from_db(row.company_id, row.user_id, row.role, row.created_at)
  }
}

pub struct PostgresCompanyMemberRepository {
  pool: PgPool,
}

impl PostgresCompanyMemberRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl CompanyMemberRepository for PostgresCompanyMemberRepository {
  async fn add_member(&self, member: CompanyMember) -> Result<CompanyMember, CompanyError> {
    let row = sqlx::query_as::<_, CompanyMemberRow>(
      r#"
            INSERT INTO company_members (company_id, user_id, role, created_at)
            VALUES ($1, $2, $3, $4)
            RETURNING company_id, user_id, role, created_at
            "#,
    )
    .bind(member.company_id)
    .bind(member.user_id)
    .bind(member.role.as_str())
    .bind(member.joined_at)
    .fetch_one(&self.pool)
    .await?;

    row.try_into()
  }

  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<CompanyMember>, CompanyError> {
    let rows = sqlx::query_as::<_, CompanyMemberRow>(
      r#"
            SELECT company_id, user_id, role, created_at
            FROM company_members
            WHERE company_id = $1
            ORDER BY created_at ASC
            "#,
    )
    .bind(company_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|row| row.try_into()).collect()
  }

  async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<CompanyMember>, CompanyError> {
    let rows = sqlx::query_as::<_, CompanyMemberRow>(
      r#"
            SELECT company_id, user_id, role, created_at
            FROM company_members
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
    )
    .bind(user_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|row| row.try_into()).collect()
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
            WHERE company_id = $1 AND user_id = $2
            "#,
    )
    .bind(company_id)
    .bind(user_id)
    .fetch_optional(&self.pool)
    .await?;

    match row {
      Some(r) => Ok(Some(r.try_into()?)),
      None => Ok(None),
    }
  }

  async fn remove_member(&self, company_id: Uuid, user_id: Uuid) -> Result<(), CompanyError> {
    sqlx::query("DELETE FROM company_members WHERE company_id = $1 AND user_id = $2")
      .bind(company_id)
      .bind(user_id)
      .execute(&self.pool)
      .await?;

    Ok(())
  }
}
