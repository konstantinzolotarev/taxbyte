use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::company::{CompanyError, CompanyMemberRepository, CompanyService};

#[derive(Debug, Clone)]
pub struct GetCompanyDetailsCommand {
  pub company_id: Uuid,
  pub requester_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompanyAddressData {
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetCompanyDetailsResponse {
  pub company_id: Uuid,
  pub name: String,
  pub email: Option<String>,
  pub phone: Option<String>,
  pub address: Option<CompanyAddressData>,
  pub registry_code: Option<String>,
  pub vat_number: Option<String>,
  pub role: String,
  pub can_edit: bool,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

pub struct GetCompanyDetailsUseCase {
  company_service: Arc<CompanyService>,
  member_repo: Arc<dyn CompanyMemberRepository>,
}

impl GetCompanyDetailsUseCase {
  pub fn new(
    company_service: Arc<CompanyService>,
    member_repo: Arc<dyn CompanyMemberRepository>,
  ) -> Self {
    Self {
      company_service,
      member_repo,
    }
  }

  pub async fn execute(
    &self,
    command: GetCompanyDetailsCommand,
  ) -> Result<GetCompanyDetailsResponse, CompanyError> {
    // Verify requester is member
    let member = self
      .member_repo
      .find_member(command.company_id, command.requester_id)
      .await?
      .ok_or(CompanyError::NotMember)?;

    // Get company
    let companies = self
      .company_service
      .get_user_companies(command.requester_id)
      .await?;
    let company = companies
      .into_iter()
      .find(|c| c.id == command.company_id)
      .ok_or(CompanyError::NotFound)?;

    // Determine permissions
    let can_edit = member.can_manage_members();
    let role = member.role.as_str().to_string();

    // Map address
    let address = company.address.as_ref().map(|a| CompanyAddressData {
      street: a.street.clone(),
      city: a.city.clone(),
      state: a.state.clone(),
      postal_code: a.postal_code.clone(),
      country: a.country.clone(),
    });

    Ok(GetCompanyDetailsResponse {
      company_id: company.id,
      name: company.name,
      email: company.email.map(|e| e.as_str().to_string()),
      phone: company.phone.map(|p| p.as_str().to_string()),
      address,
      registry_code: company.registry_code.map(|r| r.as_str().to_string()),
      vat_number: company.vat_number.map(|v| v.as_str().to_string()),
      role,
      can_edit,
      created_at: company.created_at,
      updated_at: company.updated_at,
    })
  }
}
