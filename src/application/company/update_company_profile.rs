use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::auth::value_objects::Email;
use crate::domain::company::{
  CompanyAddress, CompanyError, CompanyProfileUpdate, CompanyService, PhoneNumber, RegistryCode,
  VatNumber,
};

use super::get_company_details::CompanyAddressData;

#[derive(Debug, Clone)]
pub struct UpdateCompanyProfileCommand {
  pub company_id: Uuid,
  pub requester_id: Uuid,
  pub email: Option<String>,
  pub phone: Option<String>,
  pub address: Option<CompanyAddressData>,
  pub registry_code: Option<String>,
  pub vat_number: Option<String>,
}

pub struct UpdateCompanyProfileResponse {
  pub company_id: Uuid,
  pub name: String,
  pub updated_at: DateTime<Utc>,
}

pub struct UpdateCompanyProfileUseCase {
  company_service: Arc<CompanyService>,
}

impl UpdateCompanyProfileUseCase {
  pub fn new(company_service: Arc<CompanyService>) -> Self {
    Self { company_service }
  }

  pub async fn execute(
    &self,
    command: UpdateCompanyProfileCommand,
  ) -> Result<UpdateCompanyProfileResponse, CompanyError> {
    // Parse and validate value objects
    let email = command
      .email
      .filter(|s| !s.trim().is_empty())
      .map(Email::new)
      .transpose()
      .map_err(CompanyError::from)?;

    let phone = command
      .phone
      .filter(|s| !s.trim().is_empty())
      .map(PhoneNumber::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    let address = command
      .address
      .filter(|a| {
        a.street.is_some()
          || a.city.is_some()
          || a.state.is_some()
          || a.postal_code.is_some()
          || a.country.is_some()
      })
      .map(|a| CompanyAddress::new(a.street, a.city, a.state, a.postal_code, a.country))
      .transpose()
      .map_err(CompanyError::Validation)?;

    let registry_code = command
      .registry_code
      .filter(|s| !s.trim().is_empty())
      .map(RegistryCode::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    let vat_number = command
      .vat_number
      .filter(|s| !s.trim().is_empty())
      .map(VatNumber::new)
      .transpose()
      .map_err(CompanyError::Validation)?;

    // Create profile update struct
    let profile = CompanyProfileUpdate {
      email,
      phone,
      address,
      registry_code,
      vat_number,
    };

    // Call service
    let company = self
      .company_service
      .update_company_profile(command.company_id, command.requester_id, profile)
      .await?;

    Ok(UpdateCompanyProfileResponse {
      company_id: company.id,
      name: company.name,
      updated_at: company.updated_at,
    })
  }
}
