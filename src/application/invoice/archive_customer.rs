use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct ArchiveCustomerCommand {
  pub user_id: Uuid,
  pub customer_id: Uuid,
}

pub struct ArchiveCustomerUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl ArchiveCustomerUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(&self, command: ArchiveCustomerCommand) -> Result<(), InvoiceError> {
    self
      .invoice_service
      .archive_customer(command.user_id, command.customer_id)
      .await
  }
}
