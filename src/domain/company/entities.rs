use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

use super::errors::CompanyError;
use super::value_objects::{
  BankAccountName, BankDetails, CompanyAddress, Iban, PhoneNumber, RegistryCode, VatNumber,
};
use crate::domain::auth::value_objects::Email;

/// Company entity representing a business organization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

  /// Update company name
  pub fn update_name(&mut self, name: String) {
    self.name = name;
    self.updated_at = Utc::now();
  }

  /// Update company profile fields
  pub fn update_profile(&mut self, profile: CompanyProfileUpdate) {
    self.email = profile.email;
    self.phone = profile.phone;
    self.address = profile.address;
    self.registry_code = profile.registry_code;
    self.vat_number = profile.vat_number;
    self.updated_at = Utc::now();
  }
}

/// Company profile fields for updates
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompanyProfileUpdate {
  pub email: Option<Email>,
  pub phone: Option<PhoneNumber>,
  pub address: Option<CompanyAddress>,
  pub registry_code: Option<RegistryCode>,
  pub vat_number: Option<VatNumber>,
}

/// Company member representing user membership in a company
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    let role = CompanyRole::try_from(role.as_str())?;
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
#[non_exhaustive]
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
}

impl TryFrom<&str> for CompanyRole {
  type Error = CompanyError;

  fn try_from(s: &str) -> Result<Self, Self::Error> {
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Bank account entity representing a company's bank account
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankAccount {
  pub id: Uuid,
  pub company_id: Uuid,
  pub name: BankAccountName,
  pub iban: Iban,
  pub bank_details: Option<BankDetails>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub archived_at: Option<DateTime<Utc>>,
}

impl BankAccount {
  pub fn new(
    company_id: Uuid,
    name: BankAccountName,
    iban: Iban,
    bank_details: Option<BankDetails>,
  ) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4(),
      company_id,
      name,
      iban,
      bank_details,
      created_at: now,
      updated_at: now,
      archived_at: None,
    }
  }

  #[allow(clippy::too_many_arguments)]
  pub fn from_db(
    id: Uuid,
    company_id: Uuid,
    name: BankAccountName,
    iban: Iban,
    bank_details: Option<BankDetails>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
  ) -> Self {
    Self {
      id,
      company_id,
      name,
      iban,
      bank_details,
      created_at,
      updated_at,
      archived_at,
    }
  }

  pub fn update(&mut self, name: BankAccountName, iban: Iban, bank_details: Option<BankDetails>) {
    self.name = name;
    self.iban = iban;
    self.bank_details = bank_details;
    self.updated_at = Utc::now();
  }

  pub fn archive(&mut self) {
    self.archived_at = Some(Utc::now());
  }

  pub fn is_archived(&self) -> bool {
    self.archived_at.is_some()
  }
}

/// Active bank account for a company
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveBankAccount {
  pub company_id: Uuid,
  pub bank_account_id: Uuid,
  pub set_at: DateTime<Utc>,
}

impl ActiveBankAccount {
  pub fn new(company_id: Uuid, bank_account_id: Uuid) -> Self {
    Self {
      company_id,
      bank_account_id,
      set_at: Utc::now(),
    }
  }

  pub fn from_db(company_id: Uuid, bank_account_id: Uuid, set_at: DateTime<Utc>) -> Self {
    Self {
      company_id,
      bank_account_id,
      set_at,
    }
  }
}
