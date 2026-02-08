use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{
  entities::{ActiveBankAccount, ActiveCompany, BankAccount, Company, CompanyMember},
  errors::CompanyError,
};

#[async_trait]
pub trait CompanyRepository: Send + Sync {
  async fn create(&self, company: Company) -> Result<Company, CompanyError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<Company>, CompanyError>;
  async fn update(&self, company: Company) -> Result<Company, CompanyError>;
  async fn delete(&self, id: Uuid) -> Result<(), CompanyError>;

  /// Update OAuth tokens for a company
  async fn update_oauth_tokens(
    &self,
    company_id: &Uuid,
    encrypted_access_token: String,
    encrypted_refresh_token: String,
    expires_at: DateTime<Utc>,
    connected_by: Uuid,
  ) -> Result<(), CompanyError>;

  /// Clear OAuth tokens from a company (hard delete)
  async fn clear_oauth_tokens(&self, company_id: &Uuid) -> Result<(), CompanyError>;
}

#[async_trait]
pub trait CompanyMemberRepository: Send + Sync {
  async fn add_member(&self, member: CompanyMember) -> Result<CompanyMember, CompanyError>;
  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<CompanyMember>, CompanyError>;
  async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<CompanyMember>, CompanyError>;
  async fn find_member(
    &self,
    company_id: Uuid,
    user_id: Uuid,
  ) -> Result<Option<CompanyMember>, CompanyError>;
  async fn remove_member(&self, company_id: Uuid, user_id: Uuid) -> Result<(), CompanyError>;
}

#[async_trait]
pub trait ActiveCompanyRepository: Send + Sync {
  async fn set_active(&self, active: ActiveCompany) -> Result<(), CompanyError>;
  async fn get_active(&self, user_id: Uuid) -> Result<Option<Uuid>, CompanyError>;
  async fn clear_active(&self, user_id: Uuid) -> Result<(), CompanyError>;
}

#[async_trait]
pub trait BankAccountRepository: Send + Sync {
  async fn create(&self, account: BankAccount) -> Result<BankAccount, CompanyError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<BankAccount>, CompanyError>;
  async fn find_by_company_id(
    &self,
    company_id: Uuid,
    include_archived: bool,
  ) -> Result<Vec<BankAccount>, CompanyError>;
  async fn find_by_iban(
    &self,
    company_id: Uuid,
    iban: &str,
  ) -> Result<Option<BankAccount>, CompanyError>;
  async fn update(&self, account: BankAccount) -> Result<BankAccount, CompanyError>;
  async fn archive(&self, id: Uuid) -> Result<(), CompanyError>;
}

#[async_trait]
pub trait ActiveBankAccountRepository: Send + Sync {
  async fn set_active(&self, active: ActiveBankAccount) -> Result<(), CompanyError>;
  async fn get_active(&self, company_id: Uuid) -> Result<Option<Uuid>, CompanyError>;
  async fn clear_active(&self, company_id: Uuid) -> Result<(), CompanyError>;
}
