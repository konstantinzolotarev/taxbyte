use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct DeleteInvoiceCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
}

pub struct DeleteInvoiceUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl DeleteInvoiceUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(&self, command: DeleteInvoiceCommand) -> Result<(), InvoiceError> {
    self
      .invoice_service
      .delete_invoice(command.user_id, command.invoice_id)
      .await
  }
}
