use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{CustomerAddress, CustomerName, InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct UpdateCustomerCommand {
  pub user_id: Uuid,
  pub customer_id: Uuid,
  pub name: String,
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateCustomerResponse {
  pub customer_id: Uuid,
  pub name: String,
}

pub struct UpdateCustomerUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl UpdateCustomerUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: UpdateCustomerCommand,
  ) -> Result<UpdateCustomerResponse, InvoiceError> {
    let name = CustomerName::new(command.name)?;

    let address = if command.street.is_some()
      || command.city.is_some()
      || command.state.is_some()
      || command.postal_code.is_some()
      || command.country.is_some()
    {
      Some(CustomerAddress::new(
        command.street,
        command.city,
        command.state,
        command.postal_code,
        command.country,
      ))
    } else {
      None
    };

    let customer = self
      .invoice_service
      .update_customer(command.user_id, command.customer_id, name, address)
      .await?;

    Ok(UpdateCustomerResponse {
      customer_id: customer.id,
      name: customer.name.value().to_string(),
    })
  }
}
