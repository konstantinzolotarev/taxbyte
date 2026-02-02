use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::invoice::{InvoiceError, InvoiceService, ports::CustomerRepository};

#[derive(Debug, Deserialize)]
pub struct ListTemplatesCommand {
  pub user_id: Uuid,
  pub company_id: Uuid,
  pub include_archived: bool,
}

#[derive(Debug, Serialize)]
pub struct TemplateListItem {
  pub id: Uuid,
  pub name: String,
  pub description: Option<String>,
  pub customer_name: String,
  pub currency: String,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListTemplatesResponse {
  pub templates: Vec<TemplateListItem>,
}

pub struct ListTemplatesUseCase {
  invoice_service: Arc<InvoiceService>,
  customer_repo: Arc<dyn CustomerRepository>,
}

impl ListTemplatesUseCase {
  pub fn new(
    invoice_service: Arc<InvoiceService>,
    customer_repo: Arc<dyn CustomerRepository>,
  ) -> Self {
    Self {
      invoice_service,
      customer_repo,
    }
  }

  pub async fn execute(
    &self,
    command: ListTemplatesCommand,
  ) -> Result<ListTemplatesResponse, InvoiceError> {
    let templates = self
      .invoice_service
      .list_templates(
        command.user_id,
        command.company_id,
        command.include_archived,
      )
      .await?;

    let mut items = Vec::new();
    for template in templates {
      let customer = self
        .customer_repo
        .find_by_id(template.customer_id)
        .await?
        .ok_or(InvoiceError::CustomerNotFound(template.customer_id))?;

      items.push(TemplateListItem {
        id: template.id,
        name: template.name.into_inner(),
        description: template.description,
        customer_name: customer.name.into_inner(),
        currency: template.currency.as_str().to_string(),
        created_at: template.created_at,
      });
    }

    Ok(ListTemplatesResponse { templates: items })
  }
}
