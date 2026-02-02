use std::sync::Arc;

use uuid::Uuid;

use crate::domain::company::{BankAccountName, BankDetails, CompanyError, CompanyService, Iban};

#[derive(Debug, Clone)]
pub struct UpdateBankAccountCommand {
  pub company_id: Uuid,
  pub requester_id: Uuid,
  pub account_id: Uuid,
  pub name: String,
  pub iban: String,
  pub bank_details: Option<String>,
}

pub struct UpdateBankAccountUseCase {
  company_service: Arc<CompanyService>,
}

impl UpdateBankAccountUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(&self, command: UpdateBankAccountCommand) -> Result<(), CompanyError> {
    let name = BankAccountName::new(command.name)?;
    let iban = Iban::new(command.iban)?;
    let bank_details = command.bank_details.map(BankDetails::new).transpose()?;

    self
      .company_service
      .update_bank_account(
        command.company_id,
        command.requester_id,
        command.account_id,
        name,
        iban,
        bank_details,
      )
      .await?;

    Ok(())
  }
}
