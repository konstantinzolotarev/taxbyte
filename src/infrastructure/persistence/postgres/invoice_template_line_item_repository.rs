use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::invoice::{
  entities::InvoiceTemplateLineItem,
  errors::InvoiceError,
  ports::InvoiceTemplateLineItemRepository,
  value_objects::{Currency, LineItemDescription, Money, Quantity, VatRate},
};

#[derive(Debug, FromRow)]
struct TemplateLineItemRow {
  id: Uuid,
  template_id: Uuid,
  description: String,
  quantity: Decimal,
  unit_price_amount: Decimal,
  unit_price_currency: String,
  vat_rate: Decimal,
  line_order: i32,
}

impl TryFrom<TemplateLineItemRow> for InvoiceTemplateLineItem {
  type Error = InvoiceError;

  fn try_from(row: TemplateLineItemRow) -> Result<Self, Self::Error> {
    Ok(InvoiceTemplateLineItem {
      id: row.id,
      template_id: row.template_id,
      description: LineItemDescription::new(row.description)?,
      quantity: Quantity::new(row.quantity)?,
      unit_price: Money::new(
        row.unit_price_amount,
        Currency::from_str(&row.unit_price_currency)?,
      )?,
      vat_rate: VatRate::new(row.vat_rate)?,
      line_order: row.line_order,
    })
  }
}

pub struct PostgresInvoiceTemplateLineItemRepository {
  pool: PgPool,
}

impl PostgresInvoiceTemplateLineItemRepository {
  pub fn new(pool: PgPool) -> Self {
    Self { pool }
  }
}

#[async_trait]
impl InvoiceTemplateLineItemRepository for PostgresInvoiceTemplateLineItemRepository {
  async fn create_many(
    &self,
    items: Vec<InvoiceTemplateLineItem>,
  ) -> Result<Vec<InvoiceTemplateLineItem>, InvoiceError> {
    if items.is_empty() {
      return Ok(Vec::new());
    }

    let mut created_items = Vec::new();

    for item in items {
      let row = sqlx::query_as::<_, TemplateLineItemRow>(
        r#"
        INSERT INTO invoice_template_line_items (
          id, template_id, description, quantity,
          unit_price_amount, unit_price_currency, vat_rate, line_order
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, template_id, description, quantity,
                  unit_price_amount, unit_price_currency, vat_rate, line_order
        "#,
      )
      .bind(item.id)
      .bind(item.template_id)
      .bind(item.description.value())
      .bind(item.quantity.value())
      .bind(item.unit_price.amount)
      .bind(item.unit_price.currency.as_str())
      .bind(item.vat_rate.value())
      .bind(item.line_order)
      .fetch_one(&self.pool)
      .await?;

      created_items.push(row.try_into()?);
    }

    Ok(created_items)
  }

  async fn find_by_template_id(
    &self,
    template_id: Uuid,
  ) -> Result<Vec<InvoiceTemplateLineItem>, InvoiceError> {
    let rows = sqlx::query_as::<_, TemplateLineItemRow>(
      r#"
      SELECT id, template_id, description, quantity,
             unit_price_amount, unit_price_currency, vat_rate, line_order
      FROM invoice_template_line_items
      WHERE template_id = $1
      ORDER BY line_order ASC
      "#,
    )
    .bind(template_id)
    .fetch_all(&self.pool)
    .await?;

    rows.into_iter().map(|r| r.try_into()).collect()
  }

  async fn delete_by_template_id(&self, template_id: Uuid) -> Result<(), InvoiceError> {
    sqlx::query(
      r#"
      DELETE FROM invoice_template_line_items
      WHERE template_id = $1
      "#,
    )
    .bind(template_id)
    .execute(&self.pool)
    .await?;

    Ok(())
  }
}
