use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::domain::invoice::{
  Customer, CustomerName, errors::InvoiceError, ports::CustomerRepository,
  value_objects::CustomerAddress,
};

#[derive(Debug, FromRow)]
struct CustomerRow {
  id: Uuid,
  company_id: Uuid,
  name: String,
  address: Option<JsonValue>,
  created_at: DateTime<Utc>,
  updated_at: DateTime<Utc>,
  archived_at: Option<DateTime<Utc>>,
}

impl TryFrom<CustomerRow> for Customer {
  type Error = InvoiceError;

  fn try_from(row: CustomerRow) -> Result<Self, Self::Error> {
    let name = CustomerName::new(row.name)?;
    let address = if let Some(addr_json) = row.address {
      Some(
        serde_json::from_value::<CustomerAddress>(addr_json).map_err(|e| {
          InvoiceError::Internal(format!("Failed to parse customer address: {}", e))
        })?,
      )
    } else {
      None
    };

    Ok(Customer {
      id: row.id,
      company_id: row.company_id,
      name,
      address,
      created_at: row.created_at,
      updated_at: row.updated_at,
      archived_at: row.archived_at,
    })
  }
}

pub struct PostgresCustomerRepository {
  pool: PgPool,
}

impl PostgresCustomerRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl CustomerRepository for PostgresCustomerRepository {
  async fn create(&self, customer: Customer) -> Result<Customer, InvoiceError> {
    let address_json = customer
      .address
      .as_ref()
      .map(serde_json::to_value)
      .transpose()
      .map_err(|e| InvoiceError::Internal(format!("Failed to serialize address: {}", e)))?;

    let row = sqlx::query_as::<_, CustomerRow>(
            r#"
            INSERT INTO customers (id, company_id, name, address, created_at, updated_at, archived_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, company_id, name, address, created_at, updated_at, archived_at
            "#,
        )
        .bind(customer.id)
        .bind(customer.company_id)
        .bind(customer.name.value())
        .bind(address_json)
        .bind(customer.created_at)
        .bind(customer.updated_at)
        .bind(customer.archived_at)
        .fetch_one(&self.pool)
        .await?;

    row.try_into()
  }

  async fn update(&self, customer: Customer) -> Result<Customer, InvoiceError> {
    let address_json = customer
      .address
      .as_ref()
      .map(serde_json::to_value)
      .transpose()
      .map_err(|e| InvoiceError::Internal(format!("Failed to serialize address: {}", e)))?;

    let row = sqlx::query_as::<_, CustomerRow>(
      r#"
            UPDATE customers
            SET name = $2, address = $3, updated_at = $4, archived_at = $5
            WHERE id = $1
            RETURNING id, company_id, name, address, created_at, updated_at, archived_at
            "#,
    )
    .bind(customer.id)
    .bind(customer.name.value())
    .bind(address_json)
    .bind(customer.updated_at)
    .bind(customer.archived_at)
    .fetch_one(&self.pool)
    .await?;

    row.try_into()
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, InvoiceError> {
    let row = sqlx::query_as::<_, CustomerRow>(
      r#"
            SELECT id, company_id, name, address, created_at, updated_at, archived_at
            FROM customers
            WHERE id = $1
            "#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<Customer>, InvoiceError> {
    let rows = sqlx::query_as::<_, CustomerRow>(
      r#"
            SELECT id, company_id, name, address, created_at, updated_at, archived_at
            FROM customers
            WHERE company_id = $1
            ORDER BY name ASC
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
  ) -> Result<Vec<Customer>, InvoiceError> {
    let rows = sqlx::query_as::<_, CustomerRow>(
      r#"
            SELECT id, company_id, name, address, created_at, updated_at, archived_at
            FROM customers
            WHERE company_id = $1 AND archived_at IS NULL
            ORDER BY name ASC
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
    let result = if let Some(exclude_id) = exclude_id {
      sqlx::query_scalar::<_, bool>(
        r#"
                SELECT EXISTS(
                    SELECT 1 FROM customers
                    WHERE company_id = $1 AND name = $2 AND id != $3 AND archived_at IS NULL
                )
                "#,
      )
      .bind(company_id)
      .bind(name)
      .bind(exclude_id)
      .fetch_one(&self.pool)
      .await?
    } else {
      sqlx::query_scalar::<_, bool>(
        r#"
                SELECT EXISTS(
                    SELECT 1 FROM customers
                    WHERE company_id = $1 AND name = $2 AND archived_at IS NULL
                )
                "#,
      )
      .bind(company_id)
      .bind(name)
      .fetch_one(&self.pool)
      .await?
    };

    Ok(result)
  }
}
