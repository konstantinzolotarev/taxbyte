use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::CompanyError;
use super::value_objects::{CompanyAddress, PhoneNumber, RegistryCode, VatNumber};
use crate::domain::auth::value_objects::Email;

/// Company entity representing a business organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
  pub id: Uuid,
  pub name: String,
  pub email: Option<Email>,
  pub phone: Option<PhoneNumber>,
  pub address: Option<CompanyAddress>,
  pub registry_code: Option<RegistryCode>,
  pub vat_number: Option<VatNumber>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

impl Company {
  /// Create new company (for creation)
  pub fn new(name: String) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4(),
      name,
      email: None,
      phone: None,
      address: None,
      registry_code: None,
      vat_number: None,
      created_at: now,
      updated_at: now,
    }
  }

  /// Reconstruct from database
  pub fn from_db(
    id: Uuid,
    name: String,
    email: Option<Email>,
    phone: Option<PhoneNumber>,
    address: Option<CompanyAddress>,
    registry_code: Option<RegistryCode>,
    vat_number: Option<VatNumber>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
  ) -> Self {
    Self {
      id,
      name,
      email,
      phone,
      address,
      registry_code,
      vat_number,
      created_at,
      updated_at,
    }
  }

  /// Update company name
  pub fn update_name(&mut self, name: String) {
    self.name = name;
    self.updated_at = Utc::now();
  }

  /// Update company profile fields
  pub fn update_profile(
    &mut self,
    email: Option<Email>,
    phone: Option<PhoneNumber>,
    address: Option<CompanyAddress>,
    registry_code: Option<RegistryCode>,
    vat_number: Option<VatNumber>,
  ) {
    self.email = email;
    self.phone = phone;
    self.address = address;
    self.registry_code = registry_code;
    self.vat_number = vat_number;
    self.updated_at = Utc::now();
  }
}

/// Company member representing user membership in a company
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyMember {
  pub company_id: Uuid,
  pub user_id: Uuid,
  pub role: CompanyRole,
  pub joined_at: DateTime<Utc>,
}

impl CompanyMember {
  pub fn new(company_id: Uuid, user_id: Uuid, role: CompanyRole) -> Self {
    Self {
      company_id,
      user_id,
      role,
      joined_at: Utc::now(),
    }
  }

  pub fn from_db(
    company_id: Uuid,
    user_id: Uuid,
    role: String,
    joined_at: DateTime<Utc>,
  ) -> Result<Self, CompanyError> {
    let role = CompanyRole::from_str(&role)?;
    Ok(Self {
      company_id,
      user_id,
      role,
      joined_at,
    })
  }

  pub fn is_owner(&self) -> bool {
    matches!(self.role, CompanyRole::Owner)
  }

  pub fn can_manage_members(&self) -> bool {
    matches!(self.role, CompanyRole::Owner | CompanyRole::Admin)
  }
}

/// Company role enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanyRole {
  Owner,
  Admin,
  Member,
}

impl CompanyRole {
  pub fn as_str(&self) -> &'static str {
    match self {
      CompanyRole::Owner => "owner",
      CompanyRole::Admin => "admin",
      CompanyRole::Member => "member",
    }
  }

  pub fn from_str(s: &str) -> Result<Self, CompanyError> {
    match s.to_lowercase().as_str() {
      "owner" => Ok(CompanyRole::Owner),
      "admin" => Ok(CompanyRole::Admin),
      "member" => Ok(CompanyRole::Member),
      _ => Err(CompanyError::Validation(
        super::errors::ValidationError::InvalidRole,
      )),
    }
  }
}

/// Active company selection for a user
#[derive(Debug, Clone)]
pub struct ActiveCompany {
  pub user_id: Uuid,
  pub company_id: Uuid,
  pub set_at: DateTime<Utc>,
}

impl ActiveCompany {
  pub fn new(user_id: Uuid, company_id: Uuid) -> Self {
    Self {
      user_id,
      company_id,
      set_at: Utc::now(),
    }
  }

  pub fn from_db(user_id: Uuid, company_id: Uuid, set_at: DateTime<Utc>) -> Self {
    Self {
      user_id,
      company_id,
      set_at,
    }
  }
}
