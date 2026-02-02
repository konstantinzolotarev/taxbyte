use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValueObjectError {
  #[error("Invalid invoice number: {0}")]
  InvalidInvoiceNumber(String),
  #[error("Invalid currency code: {0}")]
  InvalidCurrency(String),
  #[error("Invalid amount: {0}")]
  InvalidAmount(String),
  #[error("Invalid line item description: {0}")]
  InvalidDescription(String),
  #[error("Invalid quantity: {0}")]
  InvalidQuantity(String),
  #[error("Invalid VAT rate: {0}")]
  InvalidVatRate(String),
  #[error("Invalid customer name: {0}")]
  InvalidCustomerName(String),
  #[error("Invalid payment terms: {0}")]
  InvalidPaymentTerms(String),
  #[error("Invalid template name: {0}")]
  InvalidTemplateName(String),
}

// Invoice Number - User-editable text field
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceNumber(String);

impl InvoiceNumber {
  pub fn new(value: String) -> Result<Self, ValueObjectError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
      return Err(ValueObjectError::InvalidInvoiceNumber(
        "Invoice number cannot be empty".to_string(),
      ));
    }
    if trimmed.len() > 100 {
      return Err(ValueObjectError::InvalidInvoiceNumber(
        "Invoice number cannot exceed 100 characters".to_string(),
      ));
    }
    Ok(Self(trimmed.to_string()))
  }

  pub fn value(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}

impl fmt::Display for InvoiceNumber {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

// Invoice Status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
  Draft,
  Sent,
  Paid,
  Overdue,
  Cancelled,
}

impl InvoiceStatus {
  pub fn can_transition_to(&self, new_status: InvoiceStatus) -> bool {
    match (self, new_status) {
      // Draft can transition to Sent or Cancelled
      (InvoiceStatus::Draft, InvoiceStatus::Sent) => true,
      (InvoiceStatus::Draft, InvoiceStatus::Cancelled) => true,
      // Sent can transition to Paid, Overdue, or Cancelled
      (InvoiceStatus::Sent, InvoiceStatus::Paid) => true,
      (InvoiceStatus::Sent, InvoiceStatus::Overdue) => true,
      (InvoiceStatus::Sent, InvoiceStatus::Cancelled) => true,
      // Overdue can transition to Paid or Cancelled
      (InvoiceStatus::Overdue, InvoiceStatus::Paid) => true,
      (InvoiceStatus::Overdue, InvoiceStatus::Cancelled) => true,
      // Paid and Cancelled are terminal states
      _ => false,
    }
  }

  pub fn is_editable(&self) -> bool {
    matches!(self, InvoiceStatus::Draft)
  }

  pub fn as_str(&self) -> &'static str {
    match self {
      InvoiceStatus::Draft => "draft",
      InvoiceStatus::Sent => "sent",
      InvoiceStatus::Paid => "paid",
      InvoiceStatus::Overdue => "overdue",
      InvoiceStatus::Cancelled => "cancelled",
    }
  }
}

impl FromStr for InvoiceStatus {
  type Err = ValueObjectError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "draft" => Ok(InvoiceStatus::Draft),
      "sent" => Ok(InvoiceStatus::Sent),
      "paid" => Ok(InvoiceStatus::Paid),
      "overdue" => Ok(InvoiceStatus::Overdue),
      "cancelled" => Ok(InvoiceStatus::Cancelled),
      _ => Err(ValueObjectError::InvalidPaymentTerms(format!(
        "Unknown status: {}",
        s
      ))),
    }
  }
}

// Currency - ISO 4217
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
  USD,
  EUR,
  GBP,
  DKK,
  SEK,
  NOK,
}

impl Currency {
  pub fn as_str(&self) -> &'static str {
    match self {
      Currency::USD => "USD",
      Currency::EUR => "EUR",
      Currency::GBP => "GBP",
      Currency::DKK => "DKK",
      Currency::SEK => "SEK",
      Currency::NOK => "NOK",
    }
  }

  pub fn symbol(&self) -> &'static str {
    match self {
      Currency::USD => "$",
      Currency::EUR => "€",
      Currency::GBP => "£",
      Currency::DKK => "kr",
      Currency::SEK => "kr",
      Currency::NOK => "kr",
    }
  }
}

impl FromStr for Currency {
  type Err = ValueObjectError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_uppercase().as_str() {
      "USD" => Ok(Currency::USD),
      "EUR" => Ok(Currency::EUR),
      "GBP" => Ok(Currency::GBP),
      "DKK" => Ok(Currency::DKK),
      "SEK" => Ok(Currency::SEK),
      "NOK" => Ok(Currency::NOK),
      _ => Err(ValueObjectError::InvalidCurrency(format!(
        "Unsupported currency: {}",
        s
      ))),
    }
  }
}

// Money - Amount with currency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
  pub amount: Decimal,
  pub currency: Currency,
}

impl Money {
  pub fn new(amount: Decimal, currency: Currency) -> Result<Self, ValueObjectError> {
    if amount.is_sign_negative() {
      return Err(ValueObjectError::InvalidAmount(
        "Amount cannot be negative".to_string(),
      ));
    }
    Ok(Self { amount, currency })
  }

  pub fn zero(currency: Currency) -> Self {
    Self {
      amount: Decimal::ZERO,
      currency,
    }
  }

  pub fn add(&self, other: &Money) -> Result<Money, ValueObjectError> {
    if self.currency != other.currency {
      return Err(ValueObjectError::InvalidAmount(
        "Cannot add amounts with different currencies".to_string(),
      ));
    }
    Ok(Money {
      amount: self.amount + other.amount,
      currency: self.currency,
    })
  }

  pub fn multiply(&self, factor: Decimal) -> Money {
    Money {
      amount: self.amount * factor,
      currency: self.currency,
    }
  }
}

impl fmt::Display for Money {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{:.2}", self.currency.symbol(), self.amount)
  }
}

// Payment Terms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentTerms {
  DueOnReceipt,
  Net15,
  Net30,
  Net60,
  Custom(i32),
}

impl PaymentTerms {
  pub fn days(&self) -> i32 {
    match self {
      PaymentTerms::DueOnReceipt => 0,
      PaymentTerms::Net15 => 15,
      PaymentTerms::Net30 => 30,
      PaymentTerms::Net60 => 60,
      PaymentTerms::Custom(days) => *days,
    }
  }

  pub fn as_str(&self) -> String {
    match self {
      PaymentTerms::DueOnReceipt => "due_on_receipt".to_string(),
      PaymentTerms::Net15 => "net_15".to_string(),
      PaymentTerms::Net30 => "net_30".to_string(),
      PaymentTerms::Net60 => "net_60".to_string(),
      PaymentTerms::Custom(days) => format!("custom_{}", days),
    }
  }
}

impl FromStr for PaymentTerms {
  type Err = ValueObjectError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "due_on_receipt" => Ok(PaymentTerms::DueOnReceipt),
      "net_15" => Ok(PaymentTerms::Net15),
      "net_30" => Ok(PaymentTerms::Net30),
      "net_60" => Ok(PaymentTerms::Net60),
      s if s.starts_with("custom_") => {
        let days_str = s.strip_prefix("custom_").ok_or_else(|| {
          ValueObjectError::InvalidPaymentTerms(format!("Invalid custom terms: {}", s))
        })?;
        let days = days_str.parse::<i32>().map_err(|_| {
          ValueObjectError::InvalidPaymentTerms(format!("Invalid custom days: {}", s))
        })?;
        if days < 0 {
          return Err(ValueObjectError::InvalidPaymentTerms(
            "Custom days must be non-negative".to_string(),
          ));
        }
        Ok(PaymentTerms::Custom(days))
      }
      _ => Err(ValueObjectError::InvalidPaymentTerms(format!(
        "Unknown payment terms: {}",
        s
      ))),
    }
  }
}

impl fmt::Display for PaymentTerms {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      PaymentTerms::DueOnReceipt => write!(f, "Due on Receipt"),
      PaymentTerms::Net15 => write!(f, "Net 15"),
      PaymentTerms::Net30 => write!(f, "Net 30"),
      PaymentTerms::Net60 => write!(f, "Net 60"),
      PaymentTerms::Custom(days) => write!(f, "Net {}", days),
    }
  }
}

// Line Item Description
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineItemDescription(String);

impl LineItemDescription {
  pub fn new(value: String) -> Result<Self, ValueObjectError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
      return Err(ValueObjectError::InvalidDescription(
        "Description cannot be empty".to_string(),
      ));
    }
    if trimmed.len() > 500 {
      return Err(ValueObjectError::InvalidDescription(
        "Description cannot exceed 500 characters".to_string(),
      ));
    }
    Ok(Self(trimmed.to_string()))
  }

  pub fn value(&self) -> &str {
    &self.0
  }
}

// Quantity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quantity(Decimal);

impl Quantity {
  pub fn new(value: Decimal) -> Result<Self, ValueObjectError> {
    if value <= Decimal::ZERO {
      return Err(ValueObjectError::InvalidQuantity(
        "Quantity must be positive".to_string(),
      ));
    }
    // Max 4 decimal places
    if value.scale() > 4 {
      return Err(ValueObjectError::InvalidQuantity(
        "Quantity cannot have more than 4 decimal places".to_string(),
      ));
    }
    Ok(Self(value))
  }

  pub fn value(&self) -> Decimal {
    self.0
  }
}

// VAT Rate
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VatRate(Decimal);

impl VatRate {
  pub fn new(value: Decimal) -> Result<Self, ValueObjectError> {
    if value < Decimal::ZERO || value > Decimal::from(100) {
      return Err(ValueObjectError::InvalidVatRate(
        "VAT rate must be between 0 and 100".to_string(),
      ));
    }
    // Max 2 decimal places
    if value.scale() > 2 {
      return Err(ValueObjectError::InvalidVatRate(
        "VAT rate cannot have more than 2 decimal places".to_string(),
      ));
    }
    Ok(Self(value))
  }

  pub fn value(&self) -> Decimal {
    self.0
  }

  pub fn as_multiplier(&self) -> Decimal {
    self.0 / Decimal::from(100)
  }
}

// Customer Name
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerName(String);

impl CustomerName {
  pub fn new(value: String) -> Result<Self, ValueObjectError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
      return Err(ValueObjectError::InvalidCustomerName(
        "Customer name cannot be empty".to_string(),
      ));
    }
    if trimmed.len() > 255 {
      return Err(ValueObjectError::InvalidCustomerName(
        "Customer name cannot exceed 255 characters".to_string(),
      ));
    }
    Ok(Self(trimmed.to_string()))
  }

  pub fn value(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}

// Template Name - User-friendly identifier for invoice templates
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateName(String);

impl TemplateName {
  pub fn new(value: String) -> Result<Self, ValueObjectError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
      return Err(ValueObjectError::InvalidTemplateName(
        "Template name cannot be empty".to_string(),
      ));
    }
    if trimmed.len() > 255 {
      return Err(ValueObjectError::InvalidTemplateName(
        "Template name cannot exceed 255 characters".to_string(),
      ));
    }
    Ok(Self(trimmed.to_string()))
  }

  pub fn value(&self) -> &str {
    &self.0
  }

  pub fn into_inner(self) -> String {
    self.0
  }
}

// Customer Address
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerAddress {
  pub street: Option<String>,
  pub city: Option<String>,
  pub state: Option<String>,
  pub postal_code: Option<String>,
  pub country: Option<String>,
}

impl CustomerAddress {
  pub fn new(
    street: Option<String>,
    city: Option<String>,
    state: Option<String>,
    postal_code: Option<String>,
    country: Option<String>,
  ) -> Self {
    Self {
      street,
      city,
      state,
      postal_code,
      country,
    }
  }

  pub fn format_multiline(&self) -> String {
    let mut lines = Vec::new();
    if let Some(street) = &self.street {
      if !street.trim().is_empty() {
        lines.push(street.clone());
      }
    }
    let mut city_line = Vec::new();
    if let Some(city) = &self.city {
      if !city.trim().is_empty() {
        city_line.push(city.clone());
      }
    }
    if let Some(state) = &self.state {
      if !state.trim().is_empty() {
        city_line.push(state.clone());
      }
    }
    if let Some(postal_code) = &self.postal_code {
      if !postal_code.trim().is_empty() {
        city_line.push(postal_code.clone());
      }
    }
    if !city_line.is_empty() {
      lines.push(city_line.join(", "));
    }
    if let Some(country) = &self.country {
      if !country.trim().is_empty() {
        lines.push(country.clone());
      }
    }
    lines.join("\n")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rust_decimal_macros::dec;

  #[test]
  fn test_invoice_number() {
    assert!(InvoiceNumber::new("INV-001".to_string()).is_ok());
    assert!(InvoiceNumber::new("".to_string()).is_err());
    assert!(InvoiceNumber::new("INV-123".to_string()).is_ok());
    assert_eq!(
      InvoiceNumber::new("INV-005".to_string()).unwrap().to_string(),
      "INV-005"
    );
  }

  #[test]
  fn test_invoice_status_transitions() {
    assert!(InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Sent));
    assert!(InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Cancelled));
    assert!(!InvoiceStatus::Draft.can_transition_to(InvoiceStatus::Paid));

    assert!(InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Paid));
    assert!(InvoiceStatus::Sent.can_transition_to(InvoiceStatus::Overdue));

    assert!(!InvoiceStatus::Paid.can_transition_to(InvoiceStatus::Sent));
    assert!(!InvoiceStatus::Cancelled.can_transition_to(InvoiceStatus::Draft));
  }

  #[test]
  fn test_currency() {
    assert_eq!(Currency::USD.as_str(), "USD");
    assert_eq!(Currency::EUR.symbol(), "€");
    assert_eq!(Currency::from_str("usd").unwrap(), Currency::USD);
    assert!(Currency::from_str("JPY").is_err());
  }

  #[test]
  fn test_money() {
    let money = Money::new(dec!(100.50), Currency::USD).unwrap();
    assert_eq!(money.amount, dec!(100.50));
    assert!(Money::new(dec!(-10), Currency::USD).is_err());
  }

  #[test]
  fn test_money_add() {
    let m1 = Money::new(dec!(100), Currency::USD).unwrap();
    let m2 = Money::new(dec!(50), Currency::USD).unwrap();
    let m3 = Money::new(dec!(50), Currency::EUR).unwrap();

    assert_eq!(m1.add(&m2).unwrap().amount, dec!(150));
    assert!(m1.add(&m3).is_err());
  }

  #[test]
  fn test_payment_terms() {
    assert_eq!(PaymentTerms::Net30.days(), 30);
    assert_eq!(PaymentTerms::Custom(45).days(), 45);
    assert_eq!(
      PaymentTerms::from_str("net_15").unwrap(),
      PaymentTerms::Net15
    );
    assert_eq!(
      PaymentTerms::from_str("custom_45").unwrap(),
      PaymentTerms::Custom(45)
    );
  }

  #[test]
  fn test_quantity() {
    assert!(Quantity::new(dec!(1)).is_ok());
    assert!(Quantity::new(dec!(0)).is_err());
    assert!(Quantity::new(dec!(-1)).is_err());
    assert!(Quantity::new(dec!(1.12345)).is_err()); // Too many decimals
  }

  #[test]
  fn test_vat_rate() {
    assert!(VatRate::new(dec!(25)).is_ok());
    assert!(VatRate::new(dec!(0)).is_ok());
    assert!(VatRate::new(dec!(100)).is_ok());
    assert!(VatRate::new(dec!(-1)).is_err());
    assert!(VatRate::new(dec!(101)).is_err());
    assert_eq!(VatRate::new(dec!(25)).unwrap().as_multiplier(), dec!(0.25));
  }

  #[test]
  fn test_customer_address() {
    let addr = CustomerAddress::new(
      Some("123 Main St".to_string()),
      Some("Copenhagen".to_string()),
      None,
      Some("1000".to_string()),
      Some("Denmark".to_string()),
    );
    let formatted = addr.format_multiline();
    assert!(formatted.contains("123 Main St"));
    assert!(formatted.contains("Copenhagen"));
    assert!(formatted.contains("Denmark"));
  }
}
