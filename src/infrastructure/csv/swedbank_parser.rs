use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::domain::report::{
  entities::ParsedTransaction, errors::ReportError, ports::BankStatementParser,
  value_objects::TransactionDirection,
};

/// Parser for Swedbank Estonia CSV bank statements
///
/// Format: semicolon-separated, quoted fields, Windows-1252 encoding
/// Columns:
///   0: Client account (IBAN)
///   1: Document number
///   2: Date (DD-MM-YYYY)
///   3: Counterparty account
///   4: Counterparty name
///   5: Counterparty bank
///   6: Empty
///   7: Direction (D/C)
///   8: Amount (comma decimal, e.g. "-1300,00")
///   9: Reference number
///  10: Archive ID
///  11: Description
///  12: Service fee
///  13: Currency
///  14: Registry code
#[derive(Default)]
pub struct SwedbankCsvParser;

impl SwedbankCsvParser {
  pub fn new() -> Self {
    Self
  }
}

impl BankStatementParser for SwedbankCsvParser {
  fn parse(&self, csv_content: &[u8]) -> Result<Vec<ParsedTransaction>, ReportError> {
    // Try to decode as Windows-1252 first, fall back to UTF-8
    let content = {
      let (decoded, _, had_errors) = encoding_rs::WINDOWS_1252.decode(csv_content);
      if had_errors {
        String::from_utf8_lossy(csv_content).into_owned()
      } else {
        decoded.into_owned()
      }
    };

    let mut reader = csv::ReaderBuilder::new()
      .delimiter(b';')
      .has_headers(true)
      .flexible(true)
      .from_reader(content.as_bytes());

    let mut transactions = Vec::new();

    for (idx, result) in reader.records().enumerate() {
      let record = result.map_err(|e| ReportError::CsvParse(format!("Row {}: {}", idx + 2, e)))?;

      // Skip empty rows
      if record.len() < 14 {
        continue;
      }

      let client_account = unquote(record.get(0).unwrap_or(""));
      if client_account.is_empty() {
        continue;
      }

      let date_str = unquote(record.get(2).unwrap_or(""));
      let date = NaiveDate::parse_from_str(&date_str, "%d-%m-%Y").map_err(|e| {
        ReportError::CsvParse(format!(
          "Row {}: invalid date '{}': {}",
          idx + 2,
          date_str,
          e
        ))
      })?;

      let direction_str = unquote(record.get(7).unwrap_or(""));
      let direction = TransactionDirection::try_from(direction_str.as_str()).map_err(|_| {
        ReportError::CsvParse(format!(
          "Row {}: invalid direction '{}'",
          idx + 2,
          direction_str
        ))
      })?;

      let amount_str = unquote(record.get(8).unwrap_or("0")).replace(',', ".");
      let amount = Decimal::from_str(&amount_str).map_err(|e| {
        ReportError::CsvParse(format!(
          "Row {}: invalid amount '{}': {}",
          idx + 2,
          amount_str,
          e
        ))
      })?;

      let counterparty_name = non_empty(record.get(4));
      let counterparty_account = non_empty(record.get(3));
      let reference_number = non_empty(record.get(9));
      let description = non_empty(record.get(11));
      let currency = unquote(record.get(13).unwrap_or("EUR"));
      let registry_code = non_empty(record.get(14));

      transactions.push(ParsedTransaction {
        row_number: (idx + 2) as i32,
        client_account,
        date,
        counterparty_name,
        counterparty_account,
        direction,
        amount,
        reference_number,
        description,
        currency: if currency.is_empty() {
          "EUR".to_string()
        } else {
          currency
        },
        registry_code,
      });
    }

    if transactions.is_empty() {
      return Err(ReportError::CsvParse(
        "No transactions found in CSV file".to_string(),
      ));
    }

    Ok(transactions)
  }
}

fn unquote(s: &str) -> String {
  s.trim().trim_matches('"').trim().to_string()
}

fn non_empty(s: Option<&str>) -> Option<String> {
  s.map(unquote).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::report::value_objects::TransactionDirection;
  use rust_decimal_macros::dec;

  fn make_csv(rows: &[&str]) -> Vec<u8> {
    let header = "\"Client account\";\"Document number\";\"Date\";\"Counterparty account\";\"Counterparty name\";\"Counterparty bank\";\"Empty\";\"D/C\";\"Amount\";\"Reference number\";\"Archive ID\";\"Description\";\"Service fee\";\"Currency\";\"Registry code\"";
    let mut lines = vec![header.to_string()];
    for row in rows {
      lines.push(row.to_string());
    }
    lines.join("\n").into_bytes()
  }

  #[test]
  fn test_parse_debit_transaction() {
    let csv = make_csv(&[
      "\"EE123456789\";\"001\";\"15-03-2026\";\"EE987654321\";\"Acme Corp\";\"HABAEE2X\";\"\";\"D\";\"-1300,00\";\"12345\";\"\";\"Office rent\";\"\";\"EUR\";\"12345678\"",
    ]);
    let parser = SwedbankCsvParser::new();
    let txs = parser.parse(&csv).unwrap();

    assert_eq!(txs.len(), 1);
    let tx = &txs[0];
    assert_eq!(tx.client_account, "EE123456789");
    assert_eq!(tx.date, NaiveDate::from_ymd_opt(2026, 3, 15).unwrap());
    assert_eq!(tx.counterparty_name.as_deref(), Some("Acme Corp"));
    assert_eq!(tx.counterparty_account.as_deref(), Some("EE987654321"));
    assert_eq!(tx.direction, TransactionDirection::Debit);
    assert_eq!(tx.amount, dec!(-1300.00));
    assert_eq!(tx.currency, "EUR");
    assert_eq!(tx.registry_code.as_deref(), Some("12345678"));
    assert_eq!(tx.reference_number.as_deref(), Some("12345"));
  }

  #[test]
  fn test_parse_credit_transaction() {
    let csv = make_csv(&[
      "\"EE123456789\";\"002\";\"16-03-2026\";\"EE111222333\";\"Client Ltd\";\"HABAEE2X\";\"\";\"C\";\"5000,50\";\"\";\"\";\"\";\"Payment received\";\"\";\"EUR\";\"\"",
    ]);
    let parser = SwedbankCsvParser::new();
    let txs = parser.parse(&csv).unwrap();

    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].direction, TransactionDirection::Credit);
    assert_eq!(txs[0].amount, dec!(5000.50));
  }

  #[test]
  fn test_parse_multiple_transactions() {
    let csv = make_csv(&[
      "\"EE123456789\";\"001\";\"15-03-2026\";\"EE987654321\";\"Acme Corp\";\"HABAEE2X\";\"\";\"D\";\"-1300,00\";\"\";\"\";\"Rent\";\"\";\"EUR\";\"\"",
      "\"EE123456789\";\"002\";\"16-03-2026\";\"EE111222333\";\"Client Ltd\";\"HABAEE2X\";\"\";\"C\";\"5000,50\";\"\";\"\";\"Income\";\"\";\"EUR\";\"\"",
    ]);
    let parser = SwedbankCsvParser::new();
    let txs = parser.parse(&csv).unwrap();
    assert_eq!(txs.len(), 2);
  }

  #[test]
  fn test_parse_usd_currency() {
    let csv = make_csv(&[
      "\"EE123456789\";\"003\";\"17-03-2026\";\"US999888777\";\"US Vendor\";\"\";\"\";\"D\";\"-250,00\";\"\";\"\";\"USD payment\";\"\";\"USD\";\"\"",
    ]);
    let parser = SwedbankCsvParser::new();
    let txs = parser.parse(&csv).unwrap();

    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].currency, "USD");
    assert_eq!(txs[0].amount, dec!(-250.00));
  }

  #[test]
  fn test_parse_empty_csv_header_only() {
    let csv = make_csv(&[]);
    let parser = SwedbankCsvParser::new();
    let result = parser.parse(&csv);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ReportError::CsvParse(_)));
  }

  #[test]
  fn test_parse_invalid_date() {
    let csv = make_csv(&[
      "\"EE123456789\";\"001\";\"2026-03-15\";\"EE987654321\";\"Acme\";\"\";\"\";\"D\";\"-100,00\";\"\";\"\";\"\";\"\";\"EUR\";\"\"",
    ]);
    let parser = SwedbankCsvParser::new();
    let result = parser.parse(&csv);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ReportError::CsvParse(_)));
  }

  #[test]
  fn test_parse_skips_short_rows() {
    let header = "\"Client account\";\"Document number\";\"Date\";\"Counterparty account\";\"Counterparty name\";\"Counterparty bank\";\"Empty\";\"D/C\";\"Amount\";\"Reference number\";\"Archive ID\";\"Description\";\"Service fee\";\"Currency\";\"Registry code\"";
    let short_row = "\"EE123\";\"001\"";
    let valid_row = "\"EE123456789\";\"002\";\"16-03-2026\";\"EE111222333\";\"Client\";\"\";\"\";\"C\";\"100,00\";\"\";\"\";\"\";\"\";\"EUR\";\"\"";
    let csv = format!("{}\n{}\n{}", header, short_row, valid_row).into_bytes();

    let parser = SwedbankCsvParser::new();
    let txs = parser.parse(&csv).unwrap();
    assert_eq!(txs.len(), 1);
  }

  #[test]
  fn test_comma_decimal_conversion() {
    let csv = make_csv(&[
      "\"EE123456789\";\"001\";\"15-03-2026\";\"\";\"\";\"\";\"\";\"D\";\"-1300,50\";\"\";\"\";\"\";\"\";\"EUR\";\"\"",
    ]);
    let parser = SwedbankCsvParser::new();
    let txs = parser.parse(&csv).unwrap();
    assert_eq!(txs[0].amount, dec!(-1300.50));
  }
}
