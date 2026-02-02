use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::InvoiceError;
use crate::domain::invoice::InvoiceService;

#[derive(Debug, Deserialize)]
pub struct ListCustomersCommand {
  pub user_id: Uuid,
  pub company_id: Uuid,
  pub include_archived: bool,
}

#[derive(Debug, Serialize)]
pub struct CustomerDto {
  pub id: Uuid,
  pub name: String,
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
  pub created_at: DateTime<Utc>,
  pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ListCustomersResponse {
  pub customers: Vec<CustomerDto>,
}

pub struct ListCustomersUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl ListCustomersUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: ListCustomersCommand,
  ) -> Result<ListCustomersResponse, InvoiceError> {
    let customers = self
      .invoice_service
      .list_customers(
        command.user_id,
        command.company_id,
        command.include_archived,
      )
      .await?;

    let customer_dtos = customers
      .into_iter()
      .map(|c| CustomerDto {
        id: c.id,
        name: c.name.value().to_string(),
        street: c.address.as_ref().and_then(|a| a.street.clone()),
        city: c.address.as_ref().and_then(|a| a.city.clone()),
        state: c.address.as_ref().and_then(|a| a.state.clone()),
        postal_code: c.address.as_ref().and_then(|a| a.postal_code.clone()),
        country: c.address.as_ref().and_then(|a| a.country.clone()),
        created_at: c.created_at,
        archived_at: c.archived_at,
      })
      .collect();

    Ok(ListCustomersResponse {
      customers: customer_dtos,
    })
  }
}
