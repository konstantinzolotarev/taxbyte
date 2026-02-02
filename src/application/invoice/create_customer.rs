use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{CustomerAddress, CustomerName, InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct CreateCustomerCommand {
  pub user_id: Uuid,
  pub company_id: Uuid,
  pub name: String,
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateCustomerResponse {
  pub customer_id: Uuid,
  pub name: String,
}

pub struct CreateCustomerUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl CreateCustomerUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: CreateCustomerCommand,
  ) -> Result<CreateCustomerResponse, InvoiceError> {
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
      .create_customer(command.user_id, command.company_id, name, address)
      .await?;

    Ok(CreateCustomerResponse {
      customer_id: customer.id,
      name: customer.name.value().to_string(),
    })
  }
}
