use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct PermanentlyDeleteInvoiceCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
}

pub struct PermanentlyDeleteInvoiceUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl PermanentlyDeleteInvoiceUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: PermanentlyDeleteInvoiceCommand,
  ) -> Result<(), InvoiceError> {
    self
      .invoice_service
      .permanently_delete_invoice(command.user_id, command.invoice_id)
      .await
  }
}
