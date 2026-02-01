use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyName, CompanyService};

#[derive(Debug, Clone)]
pub struct CreateCompanyCommand {
  pub name: String,
  pub owner_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CreateCompanyResponse {
  pub company_id: Uuid,
  pub name: String,
  pub created_at: DateTime<Utc>,
}

pub struct CreateCompanyUseCase {
  company_service: Arc<CompanyService>,
}

impl CreateCompanyUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(
    &self,
    command: CreateCompanyCommand,
  ) -> Result<CreateCompanyResponse, CompanyError> {
    let name = CompanyName::new(command.name)?;
    let company = self
      .company_service
      .create_company(name, command.owner_id)
      .await?;

    Ok(CreateCompanyResponse {
      company_id: company.id,
      name: company.name,
      created_at: company.created_at,
    })
  }
}
