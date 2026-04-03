use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct UnarchiveInvoiceCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
}

pub struct UnarchiveInvoiceUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl UnarchiveInvoiceUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(&self, command: UnarchiveInvoiceCommand) -> Result<(), InvoiceError> {
    self
      .invoice_service
      .unarchive_invoice(command.user_id, command.invoice_id)
      .await
  }
}
