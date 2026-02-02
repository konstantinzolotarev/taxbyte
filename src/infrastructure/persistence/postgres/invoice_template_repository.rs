use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  entities::InvoiceTemplate,
  errors::InvoiceError,
  ports::InvoiceTemplateRepository,
  value_objects::{Currency, PaymentTerms, TemplateName},
};

#[derive(Debug, FromRow)]
struct TemplateRow {
  id: Uuid,
  company_id: Uuid,
  name: String,
  description: Option<String>,
  customer_id: Uuid,
  bank_account_id: Option<Uuid>,
  payment_terms: String,
  currency: String,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
  archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<TemplateRow> for InvoiceTemplate {
  type Error = InvoiceError;

  fn try_from(row: TemplateRow) -> Result<Self, Self::Error> {
    Ok(InvoiceTemplate {
      id: row.id,
      company_id: row.company_id,
      name: TemplateName::new(row.name)?,
      description: row.description,
      customer_id: row.customer_id,
      bank_account_id: row.bank_account_id,
      payment_terms: PaymentTerms::from_str(&row.payment_terms)?,
      currency: Currency::from_str(&row.currency)?,
      created_at: row.created_at,
      updated_at: row.updated_at,
      archived_at: row.archived_at,
    })
  }
}

pub struct PostgresInvoiceTemplateRepository {
  pool: PgPool,
}

impl PostgresInvoiceTemplateRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceTemplateRepository for PostgresInvoiceTemplateRepository {
  async fn create(&self, template: InvoiceTemplate) -> Result<InvoiceTemplate, InvoiceError> {
    let row = sqlx::query_as::<_, TemplateRow>(
      r#"
      INSERT INTO invoice_templates (
        id, company_id, name, description, customer_id, bank_account_id,
        payment_terms, currency, created_at, updated_at, archived_at
      )
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
      RETURNING id, company_id, name, description, customer_id, bank_account_id,
                payment_terms, currency, created_at, updated_at, archived_at
      "#,
    )
    .bind(template.id)
    .bind(template.company_id)
    .bind(template.name.value())
    .bind(&template.description)
    .bind(template.customer_id)
    .bind(template.bank_account_id)
    .bind(template.payment_terms.as_str())
    .bind(template.currency.as_str())
    .bind(template.created_at)
    .bind(template.updated_at)
    .bind(template.archived_at)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      if let sqlx::Error::Database(db_err) = &e {
        if db_err.code().as_deref() == Some("23505")
          && db_err.constraint() == Some("templates_company_name_unique")
        {
          return InvoiceError::TemplateNameAlreadyExists(template.name.into_inner());
        }
      }
      InvoiceError::Database(e)
    })?;

    row.try_into()
  }

  async fn update(&self, template: InvoiceTemplate) -> Result<InvoiceTemplate, InvoiceError> {
    let row = sqlx::query_as::<_, TemplateRow>(
      r#"
      UPDATE invoice_templates
      SET name = $2, description = $3, customer_id = $4, bank_account_id = $5,
          payment_terms = $6, currency = $7, updated_at = $8, archived_at = $9
      WHERE id = $1
      RETURNING id, company_id, name, description, customer_id, bank_account_id,
                payment_terms, currency, created_at, updated_at, archived_at
      "#,
    )
    .bind(template.id)
    .bind(template.name.value())
    .bind(&template.description)
    .bind(template.customer_id)
    .bind(template.bank_account_id)
    .bind(template.payment_terms.as_str())
    .bind(template.currency.as_str())
    .bind(template.updated_at)
    .bind(template.archived_at)
    .fetch_one(&self.pool)
    .await
    .map_err(|e| {
      if let sqlx::Error::Database(db_err) = &e {
        if db_err.code().as_deref() == Some("23505")
          && db_err.constraint() == Some("templates_company_name_unique")
        {
          return InvoiceError::TemplateNameAlreadyExists(template.name.into_inner());
        }
      }
      InvoiceError::Database(e)
    })?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<InvoiceTemplate>, InvoiceError> {
    let row = sqlx::query_as::<_, TemplateRow>(
      r#"
      SELECT id, company_id, name, description, customer_id, bank_account_id,
             payment_terms, currency, created_at, updated_at, archived_at
      FROM invoice_templates
      WHERE id = $1
      "#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn find_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<InvoiceTemplate>, InvoiceError> {
    let rows = sqlx::query_as::<_, TemplateRow>(
      r#"
      SELECT id, company_id, name, description, customer_id, bank_account_id,
             payment_terms, currency, created_at, updated_at, archived_at
      FROM invoice_templates
      WHERE company_id = $1
      ORDER BY created_at DESC
      "#,
    )
    .bind(company_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn find_active_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<InvoiceTemplate>, InvoiceError> {
    let rows = sqlx::query_as::<_, TemplateRow>(
      r#"
      SELECT id, company_id, name, description, customer_id, bank_account_id,
             payment_terms, currency, created_at, updated_at, archived_at
      FROM invoice_templates
      WHERE company_id = $1 AND archived_at IS NULL
      ORDER BY created_at DESC
      "#,
    )
    .bind(company_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn exists_by_name(
    &self,
    company_id: Uuid,
    name: &str,
    exclude_id: Option<Uuid>,
  ) -> Result<bool, InvoiceError> {
    let query = if let Some(id) = exclude_id {
      sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1 FROM invoice_templates
          WHERE company_id = $1 AND name = $2 AND id != $3
        )
        "#,
      )
      .bind(company_id)
      .bind(name)
      .bind(id)
    } else {
      sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
          SELECT 1 FROM invoice_templates
          WHERE company_id = $1 AND name = $2
        )
        "#,
      )
      .bind(company_id)
      .bind(name)
    };

    Ok(query.fetch_one(&self.pool).await?)
  }
}
