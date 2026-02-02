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

/// Bank account name value object with validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankAccountName(String);

impl BankAccountName {
  const MIN_LENGTH: usize = 1;
  const MAX_LENGTH: usize = 100;

  pub fn new(name: impl Into<String>) -> Result<Self, ValidationError> {
    let name = name.into().trim().to_string();

    if name.is_empty() {
      return Err(ValidationError::BankAccountNameTooShort {
        min: Self::MIN_LENGTH,
      });
    }

    if name.len() > Self::MAX_LENGTH {
      return Err(ValidationError::BankAccountNameTooLong {
        max: Self::MAX_LENGTH,
      });
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

/// IBAN (International Bank Account Number) value object with strict validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Iban(String);

impl Iban {
  const MIN_LENGTH: usize = 15;
  const MAX_LENGTH: usize = 34;

  pub fn new(iban: impl Into<String>) -> Result<Self, ValidationError> {
    let iban = iban
      .into()
      .chars()
      .filter(|c| !c.is_whitespace())
      .collect::<String>()
      .to_uppercase();

    // Validate length
    if iban.len() < Self::MIN_LENGTH || iban.len() > Self::MAX_LENGTH {
      return Err(ValidationError::IbanInvalidLength {
        min: Self::MIN_LENGTH,
        max: Self::MAX_LENGTH,
      });
    }

    // Validate format: 2 letters (country code) + 2 digits (check digits) + alphanumeric
    if !Self::is_valid_format(&iban) {
      return Err(ValidationError::IbanInvalidFormat);
    }

    // Validate checksum using mod-97 algorithm
    if !Self::is_valid_checksum(&iban) {
      return Err(ValidationError::IbanInvalidChecksum);
    }

    Ok(Self(iban))
  }

  fn is_valid_format(iban: &str) -> bool {
    let chars: Vec<char> = iban.chars().collect();

    if chars.len() < 4 {
      return false;
    }

    // First 2 characters must be letters (country code)
    if !chars[0].is_ascii_alphabetic() || !chars[1].is_ascii_alphabetic() {
      return false;
    }

    // Next 2 characters must be digits (check digits)
    if !chars[2].is_ascii_digit() || !chars[3].is_ascii_digit() {
      return false;
    }

    // Remaining characters must be alphanumeric
    chars[4..].iter().all(|c| c.is_ascii_alphanumeric())
  }

  fn is_valid_checksum(iban: &str) -> bool {
    // Move first 4 characters to the end
    let rearranged = format!("{}{}", &iban[4..], &iban[..4]);

    // Convert letters to numbers (A=10, B=11, ..., Z=35)
    let numeric = rearranged
      .chars()
      .map(|c| {
        if c.is_ascii_digit() {
          c.to_string()
        } else {
          // A=10, B=11, ..., Z=35
          ((c as u32) - ('A' as u32) + 10).to_string()
        }
      })
      .collect::<String>();

    // Calculate mod 97
    Self::mod97(&numeric) == 1
  }

  // Calculate mod 97 for large numbers (as strings)
  fn mod97(number: &str) -> u32 {
    let mut remainder = 0u32;

    for digit in number.chars() {
      let digit_value = digit.to_digit(10).unwrap();
      remainder = (remainder * 10 + digit_value) % 97;
    }

    remainder
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }

  /// Format IBAN with spaces every 4 characters for display
  pub fn formatted(&self) -> String {
    self
      .0
      .chars()
      .enumerate()
      .fold(String::new(), |mut acc, (i, c)| {
        if i > 0 && i % 4 == 0 {
          acc.push(' ');
        }
        acc.push(c);
        acc
      })
  }
}

/// Bank details value object (optional textarea)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankDetails(String);

impl BankDetails {
  const MAX_LENGTH: usize = 1000;

  pub fn new(details: impl Into<String>) -> Result<Self, ValidationError> {
    let details = details.into().trim().to_string();

    if details.len() > Self::MAX_LENGTH {
      return Err(ValidationError::BankDetailsTooLong {
        max: Self::MAX_LENGTH,
      });
    }

    Ok(Self(details))
  }

  pub fn as_str(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }
}
