use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService};

#[derive(Debug, Deserialize)]
pub struct ArchiveTemplateCommand {
  pub user_id: Uuid,
  pub template_id: Uuid,
}

pub struct ArchiveTemplateUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl ArchiveTemplateUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(&self, command: ArchiveTemplateCommand) -> Result<(), InvoiceError> {
    self
      .invoice_service
      .archive_template(command.user_id, command.template_id)
      .await
  }
}
