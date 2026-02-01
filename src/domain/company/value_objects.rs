use serde::{Deserialize, Serialize};

use super::errors::ValidationError;

/// Company name value object with validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanyName(String);

impl CompanyName {
  pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
    let name = name.into().trim().to_string();

    if name.is_empty() {
      return Err(ValidationError::CompanyNameTooShort { min: 1 });
    }

    if name.len() > 255 {
      return Err(ValidationError::CompanyNameTooLong { max: 255 });
    }

    Ok(Self(name))
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}

/// Company address value object (structured)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyAddress {
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
}

impl CompanyAddress {
  const MAX_FIELD_LENGTH: usize = 255;

  pub fn new(
    street: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
  ) -> Result<Self, ValidationError> {
    // Validate and trim each field
    let street = Self::validate_field(street, "street")?;
    let city = Self::validate_field(city, "city")?;
    let state = Self::validate_field(state, "state")?;
    let postal_code = Self::validate_field(postal_code, "postal_code")?;
    let country = Self::validate_field(country, "country")?;

    Ok(Self {
      street,
      city,
      state,
      postal_code,
      country,
    })
  }

  fn validate_field(
    field: Option<String>,
    field_name: &str,
  ) -> Result<Option<String>, ValidationError> {
    match field {
      Some(s) => {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
          Ok(None)
        } else if trimmed.len() > Self::MAX_FIELD_LENGTH {
          Err(ValidationError::AddressFieldTooLong {
            field: field_name.to_string(),
            max: Self::MAX_FIELD_LENGTH,
          })
        } else {
          Ok(Some(trimmed))
        }
      }
      None => Ok(None),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.street.is_none()
      && self.city.is_none()
      && self.state.is_none()
      && self.postal_code.is_none()
      && self.country.is_none()
  }

  pub fn as_json(&self) -> Result<String, serde_json::Error> {
    serde_json::to_string(self)
  }

  pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
    serde_json::from_str(json)
  }
}

/// Phone number value object with basic validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneNumber(String);

impl PhoneNumber {
  const MIN_LENGTH: usize = 10;
  const MAX_LENGTH: usize = 20;

  pub fn new(phone: impl Into<String>) -> Result<Self, ValidationError> {
    let phone = phone.into().trim().to_string();

    if phone.len() < Self::MIN_LENGTH || phone.len() > Self::MAX_LENGTH {
      return Err(ValidationError::PhoneNumberInvalidLength {
        min: Self::MIN_LENGTH,
        max: Self::MAX_LENGTH,
      });
    }

    // Validate characters: digits, spaces, +, -, (, )
    if !phone
      .chars()
      .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '+' | '-' | '(' | ')'))
    {
      return Err(ValidationError::PhoneNumberInvalidCharacters);
    }

    Ok(Self(phone))
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}

/// Registry code value object with minimal validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryCode(String);

impl RegistryCode {
  const MAX_LENGTH: usize = 50;

  pub fn new(code: impl Into<String>) -> Result<Self, ValidationError> {
    let code = code.into().trim().to_string();

    if code.len() > Self::MAX_LENGTH {
      return Err(ValidationError::RegistryCodeTooLong {
        max: Self::MAX_LENGTH,
      });
    }

    Ok(Self(code))
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}

/// VAT number value object with minimal validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VatNumber(String);

impl VatNumber {
  const MAX_LENGTH: usize = 50;

  pub fn new(vat: impl Into<String>) -> Result<Self, ValidationError> {
    let vat = vat.into().trim().to_string();

    if vat.len() > Self::MAX_LENGTH {
      return Err(ValidationError::VatNumberTooLong {
        max: Self::MAX_LENGTH,
      });
    }

    Ok(Self(vat))
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}
