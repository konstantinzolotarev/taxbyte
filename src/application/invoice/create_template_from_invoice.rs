use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService, TemplateName};

#[derive(Debug, Deserialize)]
pub struct CreateTemplateFromInvoiceCommand {
  pub user_id: Uuid,
  pub invoice_id: Uuid,
  pub template_name: String,
  pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTemplateFromInvoiceResponse {
  pub template_id: Uuid,
  pub name: String,
  pub created_at: DateTime<Utc>,
}

pub struct CreateTemplateFromInvoiceUseCase {
  invoice_service: Arc<InvoiceService>,
}

impl CreateTemplateFromInvoiceUseCase {
  pub fn new(invoice_service: Arc<InvoiceService>) -> Self {
    Self { invoice_service }
  }

  pub async fn execute(
    &self,
    command: CreateTemplateFromInvoiceCommand,
  ) -> Result<CreateTemplateFromInvoiceResponse, InvoiceError> {
    let template_name = TemplateName::new(command.template_name)?;

    let (template, _items) = self
      .invoice_service
      .create_template_from_invoice(
        command.user_id,
        command.invoice_id,
        template_name,
        command.description,
      )
      .await?;

    Ok(CreateTemplateFromInvoiceResponse {
      template_id: template.id,
      name: template.name.into_inner(),
      created_at: template.created_at,
    })
  }
}
