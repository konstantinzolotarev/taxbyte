use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  Currency, InvoiceLineItem, LineItemDescription, Money, Quantity, VatRate, errors::InvoiceError,
  ports::InvoiceLineItemRepository,
};

#[derive(Debug, FromRow)]
struct LineItemRow {
  id: Uuid,
  invoice_id: Uuid,
  description: String,
  quantity: Decimal,
  unit_price_amount: Decimal,
  unit_price_currency: String,
  vat_rate: Decimal,
  line_order: i32,
}

impl TryFrom<LineItemRow> for InvoiceLineItem {
  type Error = InvoiceError;

  fn try_from(row: LineItemRow) -> Result<Self, Self::Error> {
    let description = LineItemDescription::new(row.description)?;
    let quantity = Quantity::new(row.quantity)?;
    let currency = Currency::from_str(&row.unit_price_currency)?;
    let unit_price = Money::new(row.unit_price_amount, currency)?;
    let vat_rate = VatRate::new(row.vat_rate)?;

    Ok(InvoiceLineItem {
      id: row.id,
      invoice_id: row.invoice_id,
      description,
      quantity,
      unit_price,
      vat_rate,
      line_order: row.line_order,
    })
  }
}

pub struct PostgresInvoiceLineItemRepository {
  pool: PgPool,
}

impl PostgresInvoiceLineItemRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceLineItemRepository for PostgresInvoiceLineItemRepository {
  async fn create(&self, line_item: InvoiceLineItem) -> Result<InvoiceLineItem, InvoiceError> {
    let row = sqlx::query_as::<_, LineItemRow>(
      r#"
            INSERT INTO invoice_line_items (
                id, invoice_id, description, quantity,
                unit_price_amount, unit_price_currency, vat_rate, line_order
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, invoice_id, description, quantity,
                      unit_price_amount, unit_price_currency, vat_rate, line_order
            "#,
    )
    .bind(line_item.id)
    .bind(line_item.invoice_id)
    .bind(line_item.description.value())
    .bind(line_item.quantity.value())
    .bind(line_item.unit_price.amount)
    .bind(line_item.unit_price.currency.as_str())
    .bind(line_item.vat_rate.value())
    .bind(line_item.line_order)
    .fetch_one(&self.pool)
    .await?;

    row.try_into()
  }

  async fn create_many(
    &self,
    line_items: Vec<InvoiceLineItem>,
  ) -> Result<Vec<InvoiceLineItem>, InvoiceError> {
    let mut created_items = Vec::new();

    for item in line_items {
      let created = self.create(item).await?;
      created_items.push(created);
    }

    Ok(created_items)
  }

  async fn update(&self, line_item: InvoiceLineItem) -> Result<InvoiceLineItem, InvoiceError> {
    let row = sqlx::query_as::<_, LineItemRow>(
      r#"
            UPDATE invoice_line_items
            SET description = $2, quantity = $3, unit_price_amount = $4,
                unit_price_currency = $5, vat_rate = $6, line_order = $7
            WHERE id = $1
            RETURNING id, invoice_id, description, quantity,
                      unit_price_amount, unit_price_currency, vat_rate, line_order
            "#,
    )
    .bind(line_item.id)
    .bind(line_item.description.value())
    .bind(line_item.quantity.value())
    .bind(line_item.unit_price.amount)
    .bind(line_item.unit_price.currency.as_str())
    .bind(line_item.vat_rate.value())
    .bind(line_item.line_order)
    .fetch_one(&self.pool)
    .await?;

    row.try_into()
  }

  async fn delete(&self, id: Uuid) -> Result<(), InvoiceError> {
    sqlx::query("DELETE FROM invoice_line_items WHERE id = $1")
      .bind(id)
      .execute(&self.pool)
      .await?;

    Ok(())
  }

  async fn delete_by_invoice_id(&self, invoice_id: Uuid) -> Result<(), InvoiceError> {
    sqlx::query("DELETE FROM invoice_line_items WHERE invoice_id = $1")
      .bind(invoice_id)
      .execute(&self.pool)
      .await?;

    Ok(())
  }

  async fn find_by_id(&self, id: Uuid) -> Result<Option<InvoiceLineItem>, InvoiceError> {
    let row = sqlx::query_as::<_, LineItemRow>(
      r#"
            SELECT id, invoice_id, description, quantity,
                   unit_price_amount, unit_price_currency, vat_rate, line_order
            FROM invoice_line_items
            WHERE id = $1
            "#,
    )
    .bind(id)
    .fetch_optional(&self.pool)
    .await?;

    row.map(|r| r.try_into()).transpose()
  }

  async fn find_by_invoice_id(
    &self,
    invoice_id: Uuid,
  ) -> Result<Vec<InvoiceLineItem>, InvoiceError> {
    let rows = sqlx::query_as::<_, LineItemRow>(
      r#"
            SELECT id, invoice_id, description, quantity,
                   unit_price_amount, unit_price_currency, vat_rate, line_order
            FROM invoice_line_items
            WHERE invoice_id = $1
            ORDER BY line_order ASC
            "#,
    )
    .bind(invoice_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }
}
