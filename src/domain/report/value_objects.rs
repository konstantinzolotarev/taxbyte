use serde::{Deserialize, Serialize};

use super::errors::ReportError;

/// Validated month/year pair
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportMonth {
  pub month: u32,
  pub year: i32,
}

impl ReportMonth {
  pub fn new(month: u32, year: i32) -> Result<Self, ReportError> {
    if !(1..=12).contains(&month) {
      return Err(ReportError::Validation(format!(
        "Invalid month: {}. Must be 1-12",
        month
      )));
    }
    if !(2000..=2100).contains(&year) {
      return Err(ReportError::Validation(format!(
        "Invalid year: {}. Must be 2000-2100",
        year
      )));
    }
    Ok(Self { month, year })
  }

  /// Returns folder name like "03.2026"
  pub fn folder_name(&self) -> String {
    format!("{:02}.{}", self.month, self.year)
  }
}

/// Direction of a bank transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionDirection {
  Debit,
  Credit,
}

impl TransactionDirection {
  pub fn as_str(&self) -> &'static str {
    match self {
      TransactionDirection::Debit => "debit",
      TransactionDirection::Credit => "credit",
    }
  }
}

impl TryFrom<&str> for TransactionDirection {
  type Error = ReportError;

  fn try_from(s: &str) -> Result<Self, Self::Error> {
    match s.to_uppercase().as_str() {
      "D" | "DEBIT" => Ok(TransactionDirection::Debit),
      "C" | "CREDIT" => Ok(TransactionDirection::Credit),
      _ => Err(ReportError::Validation(format!(
        "Invalid transaction direction: '{}'. Expected 'D' or 'C'",
        s
      ))),
    }
  }
}

/// Status of a monthly report
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportStatus {
  Draft,
  Generated,
}

impl ReportStatus {
  pub fn as_str(&self) -> &'static str {
    match self {
      ReportStatus::Draft => "draft",
      ReportStatus::Generated => "generated",
    }
  }
}

impl TryFrom<&str> for ReportStatus {
  type Error = ReportError;

  fn try_from(s: &str) -> Result<Self, Self::Error> {
    match s.to_lowercase().as_str() {
      "draft" => Ok(ReportStatus::Draft),
      "generated" => Ok(ReportStatus::Generated),
      _ => Err(ReportError::Validation(format!(
        "Invalid report status: '{}'",
        s
      ))),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ReportMonth tests

  #[test]
  fn test_report_month_valid() {
    for month in 1..=12 {
      let rm = ReportMonth::new(month, 2026).unwrap();
      assert_eq!(rm.month, month);
      assert_eq!(rm.year, 2026);
    }
  }

  #[test]
  fn test_report_month_invalid_month_zero() {
    assert!(ReportMonth::new(0, 2026).is_err());
  }

  #[test]
  fn test_report_month_invalid_month_thirteen() {
    assert!(ReportMonth::new(13, 2026).is_err());
  }

  #[test]
  fn test_report_month_invalid_year_too_low() {
    assert!(ReportMonth::new(1, 1999).is_err());
  }

  #[test]
  fn test_report_month_invalid_year_too_high() {
    assert!(ReportMonth::new(1, 2101).is_err());
  }

  #[test]
  fn test_report_month_boundary_years() {
    assert!(ReportMonth::new(1, 2000).is_ok());
    assert!(ReportMonth::new(1, 2100).is_ok());
  }

  #[test]
  fn test_report_month_folder_name_with_leading_zero() {
    let rm = ReportMonth::new(3, 2026).unwrap();
    assert_eq!(rm.folder_name(), "03.2026");
  }

  #[test]
  fn test_report_month_folder_name_without_leading_zero() {
    let rm = ReportMonth::new(12, 2026).unwrap();
    assert_eq!(rm.folder_name(), "12.2026");
  }

  // TransactionDirection tests

  #[test]
  fn test_transaction_direction_as_str() {
    assert_eq!(TransactionDirection::Debit.as_str(), "debit");
    assert_eq!(TransactionDirection::Credit.as_str(), "credit");
  }

  #[test]
  fn test_transaction_direction_try_from_d_c() {
    assert_eq!(
      TransactionDirection::try_from("D").unwrap(),
      TransactionDirection::Debit
    );
    assert_eq!(
      TransactionDirection::try_from("C").unwrap(),
      TransactionDirection::Credit
    );
  }

  #[test]
  fn test_transaction_direction_try_from_full_words() {
    assert_eq!(
      TransactionDirection::try_from("debit").unwrap(),
      TransactionDirection::Debit
    );
    assert_eq!(
      TransactionDirection::try_from("credit").unwrap(),
      TransactionDirection::Credit
    );
  }

  #[test]
  fn test_transaction_direction_case_insensitive() {
    assert_eq!(
      TransactionDirection::try_from("d").unwrap(),
      TransactionDirection::Debit
    );
    assert_eq!(
      TransactionDirection::try_from("CREDIT").unwrap(),
      TransactionDirection::Credit
    );
  }

  #[test]
  fn test_transaction_direction_invalid() {
    assert!(TransactionDirection::try_from("X").is_err());
    assert!(TransactionDirection::try_from("").is_err());
  }

  // ReportStatus tests

  #[test]
  fn test_report_status_as_str() {
    assert_eq!(ReportStatus::Draft.as_str(), "draft");
    assert_eq!(ReportStatus::Generated.as_str(), "generated");
  }

  #[test]
  fn test_report_status_try_from_valid() {
    assert_eq!(
      ReportStatus::try_from("draft").unwrap(),
      ReportStatus::Draft
    );
    assert_eq!(
      ReportStatus::try_from("generated").unwrap(),
      ReportStatus::Generated
    );
    assert_eq!(
      ReportStatus::try_from("DRAFT").unwrap(),
      ReportStatus::Draft
    );
  }

  #[test]
  fn test_report_status_try_from_invalid() {
    assert!(ReportStatus::try_from("pending").is_err());
    assert!(ReportStatus::try_from("").is_err());
  }
}
