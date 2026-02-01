use async_trait::async_trait;
use uuid::Uuid;

use super::{
  entities::{ActiveCompany, Company, CompanyMember},
  errors::CompanyError,
};

#[async_trait]
pub trait CompanyRepository: Send + Sync {
  async fn create(&self, company: Company) -> Result<Company, CompanyError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<Company>, CompanyError>;
  async fn update(&self, company: Company) -> Result<Company, CompanyError>;
  async fn delete(&self, id: Uuid) -> Result<(), CompanyError>;
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
