use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::company::{BankAccountName, BankDetails, CompanyError, CompanyService, Iban};

#[derive(Debug, Clone)]
pub struct CreateBankAccountCommand {
  pub company_id: Uuid,
  pub requester_id: Uuid,
  pub name: String,
  pub iban: String,
  pub bank_details: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateBankAccountResponse {
  pub id: Uuid,
  pub company_id: Uuid,
  pub name: String,
  pub iban: String,
  pub bank_details: Option<String>,
  pub created_at: DateTime<Utc>,
}

pub struct CreateBankAccountUseCase {
  company_service: Arc<CompanyService>,
}

impl CreateBankAccountUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(
    &self,
    command: CreateBankAccountCommand,
  ) -> Result<CreateBankAccountResponse, CompanyError> {
    let name = BankAccountName::new(command.name)?;
    let iban = Iban::new(command.iban)?;
    let bank_details = command.bank_details.map(BankDetails::new).transpose()?;

    let account = self
      .company_service
      .create_bank_account(
        command.company_id,
        command.requester_id,
        name,
        iban,
        bank_details,
      )
      .await?;

    Ok(CreateBankAccountResponse {
      id: account.id,
      company_id: account.company_id,
      name: account.name.into_inner(),
      iban: account.iban.into_inner(),
      bank_details: account.bank_details.map(|d| d.into_inner()),
      created_at: account.created_at,
    })
  }
}
