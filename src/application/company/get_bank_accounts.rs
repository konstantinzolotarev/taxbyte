use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyService};

#[derive(Debug, Clone)]
pub struct GetBankAccountsCommand {
  pub company_id: Uuid,
  pub requester_id: Uuid,
  pub include_archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccountDto {
  pub id: Uuid,
  pub company_id: Uuid,
  pub name: String,
  pub iban: String,
  pub iban_formatted: String,
  pub bank_details: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct GetBankAccountsResponse {
  pub accounts: Vec<BankAccountDto>,
}

pub struct GetBankAccountsUseCase {
  company_service: Arc<CompanyService>,
}

impl GetBankAccountsUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(
    &self,
    command: GetBankAccountsCommand,
  ) -> Result<GetBankAccountsResponse, CompanyError> {
    let accounts = self
      .company_service
      .get_company_bank_accounts(
        command.company_id,
        command.requester_id,
        command.include_archived,
      )
      .await?;

    let dtos = accounts
      .into_iter()
      .map(|account| BankAccountDto {
        id: account.id,
        company_id: account.company_id,
        iban_formatted: account.iban.formatted(),
        name: account.name.as_str().to_string(),
        iban: account.iban.into_inner(),
        bank_details: account.bank_details.map(|d| d.into_inner()),
        created_at: account.created_at,
        updated_at: account.updated_at,
        archived_at: account.archived_at,
      })
      .collect();

    Ok(GetBankAccountsResponse { accounts: dtos })
  }
}
