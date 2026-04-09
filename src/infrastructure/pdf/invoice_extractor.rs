use regex::Regex;

use crate::domain::report::{ExtractedInvoiceData, InvoiceDataExtractor, ReportError};

#[derive(Default)]
pub struct PdfInvoiceExtractor;

impl PdfInvoiceExtractor {
  pub fn new() -> Self {
    Self
  }
}

impl InvoiceDataExtractor for PdfInvoiceExtractor {
  fn extract(&self, pdf_bytes: &[u8]) -> Result<ExtractedInvoiceData, ReportError> {
    let text = match pdf_extract::extract_text_from_mem(pdf_bytes) {
      Ok(t) => t,
      Err(e) => {
        tracing::debug!("PDF text extraction failed: {}", e);
        return Ok(ExtractedInvoiceData::default());
      }
    };

    if text.trim().is_empty() {
      return Ok(ExtractedInvoiceData::default());
    }

    Ok(ExtractedInvoiceData {
      amount: extract_amount(&text),
      currency: extract_currency(&text),
      vendor_name: extract_vendor_name(&text),
      invoice_number: extract_invoice_number(&text),
      invoice_date: extract_invoice_date(&text),
    })
  }
}

/// Extract the total amount from invoice text.
/// Looks for amounts near keywords like "total", "summa", "kokku", "tasuda", "amount due".
/// Prioritizes specific grand-total keywords first, then falls back to generic ones
/// (using the last occurrence to prefer the final total over subtotals).
fn extract_amount(text: &str) -> Option<String> {
  // Specific grand-total keywords — checked first, first occurrence wins
  let specific_keywords = [
    "arve summa kokku",
    "grand total",
    "invoice total",
    "total to pay",
    "total amount",
    "amount due",
    "total due",
    "amount payable",
    "summa kokku",
    "kokku tasuda",
    "к оплате",
  ];

  // Generic keywords — checked second, LAST occurrence wins (grand total is usually at bottom)
  let generic_keywords = ["tasuda", "kokku", "total", "summa", "итого"];

  let amount_re = Regex::new(r"(\d[\d\s]*[.,]\d{2})\b").unwrap();
  let text_lower = text.to_lowercase();

  // Pass 1: specific keywords — first match wins
  for keyword in &specific_keywords {
    if let Some(amount) = find_amount_near_keyword(&text_lower, text, keyword, &amount_re) {
      return Some(amount);
    }
  }

  // Pass 2: generic keywords — use LAST occurrence (bottom of document = grand total)
  for keyword in &generic_keywords {
    if let Some(amount) = find_amount_near_last_keyword(&text_lower, text, keyword, &amount_re) {
      return Some(amount);
    }
  }

  // Fallback: find the largest amount in the document
  let mut largest: Option<(f64, String)> = None;
  for m in amount_re.find_iter(text) {
    let amount = normalize_amount(m.as_str());
    if let Ok(val) = amount.parse::<f64>() {
      if val > 0.0 {
        match &largest {
          Some((prev, _)) if val > *prev => largest = Some((val, amount)),
          None => largest = Some((val, amount)),
          _ => {}
        }
      }
    }
  }

  largest.map(|(_, s)| s)
}

/// Find an amount near the first occurrence of a keyword.
fn find_amount_near_keyword(
  text_lower: &str,
  text: &str,
  keyword: &str,
  amount_re: &Regex,
) -> Option<String> {
  let pos = text_lower.find(keyword)?;
  extract_amount_after_pos(text, pos + keyword.len(), amount_re)
}

/// Find an amount near the LAST occurrence of a keyword.
fn find_amount_near_last_keyword(
  text_lower: &str,
  text: &str,
  keyword: &str,
  amount_re: &Regex,
) -> Option<String> {
  let pos = text_lower.rfind(keyword)?;
  extract_amount_after_pos(text, pos + keyword.len(), amount_re)
}

/// Extract the first valid amount within 100 chars after a position.
fn extract_amount_after_pos(text: &str, start: usize, amount_re: &Regex) -> Option<String> {
  let search_end = (start + 100).min(text.len());
  let search_slice = &text[start..search_end];
  let m = amount_re.find(search_slice)?;
  let amount = normalize_amount(m.as_str());
  if let Ok(val) = amount.parse::<f64>() {
    if val > 0.0 {
      return Some(amount);
    }
  }
  None
}

/// Normalize an amount string: remove spaces, replace comma with dot
fn normalize_amount(s: &str) -> String {
  let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
  cleaned.replace(',', ".")
}

/// Extract currency from invoice text.
fn extract_currency(text: &str) -> Option<String> {
  let text_upper = text.to_uppercase();

  // Check for currency symbols first
  if text.contains('€') || text_upper.contains("EUR") {
    return Some("EUR".to_string());
  }
  if text.contains('$') || text_upper.contains("USD") {
    return Some("USD".to_string());
  }
  if text.contains('£') || text_upper.contains("GBP") {
    return Some("GBP".to_string());
  }
  if text_upper.contains("SEK") {
    return Some("SEK".to_string());
  }

  None
}

/// Extract vendor name from invoice text.
/// Tries to find text near "From:", "Seller:", "Müüja:", or falls back to
/// the first meaningful non-numeric line near the top.
fn extract_vendor_name(text: &str) -> Option<String> {
  let seller_keywords = [
    "seller:",
    "from:",
    "müüja:",
    "vendor:",
    "supplier:",
    "billed by:",
    "company:",
    "поставщик:",
    "продавец:",
  ];

  let text_lower = text.to_lowercase();

  for keyword in &seller_keywords {
    if let Some(pos) = text_lower.find(keyword) {
      let after = &text[pos + keyword.len()..];
      // Take text until end of line
      let name = after
        .lines()
        .next()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .or_else(|| {
          // If same line is empty, take the next non-empty line
          after
            .lines()
            .skip(1)
            .find(|l| !l.trim().is_empty())
            .map(|l| l.trim())
        });

      if let Some(name) = name {
        let cleaned = clean_vendor_name(name);
        if !cleaned.is_empty() {
          return Some(cleaned);
        }
      }
    }
  }

  // Fallback: first meaningful line at the top of the document
  for line in text.lines().take(10) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      continue;
    }
    // Skip lines that look like dates, numbers, or common header words
    if looks_like_header_noise(trimmed) {
      continue;
    }
    let cleaned = clean_vendor_name(trimmed);
    if cleaned.len() >= 3 && cleaned.len() <= 100 {
      return Some(cleaned);
    }
  }

  None
}

fn clean_vendor_name(s: &str) -> String {
  s.trim_matches(|c: char| c == ':' || c == ',' || c.is_whitespace())
    .to_string()
}

fn looks_like_header_noise(s: &str) -> bool {
  let lower = s.to_lowercase();
  // Skip lines that are just numbers, dates, or common invoice header words
  if s
    .chars()
    .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '/' || c == ' ')
  {
    return true;
  }
  let noise_words = [
    "invoice",
    "arve",
    "bill",
    "receipt",
    "credit note",
    "date",
    "kuupäev",
    "page",
  ];
  noise_words.iter().any(|w| lower == *w)
}

/// Extract invoice number from text.
fn extract_invoice_number(text: &str) -> Option<String> {
  let patterns = [
    r"(?i)(?:invoice|arve|inv)[.\s#:nNrR°]*\s*([A-Za-z0-9][\w\-/]{1,30})",
    r"(?i)(?:number|nr|no)[.\s#:]*\s*([A-Za-z0-9][\w\-/]{1,30})",
  ];

  for pattern in &patterns {
    let re = Regex::new(pattern).unwrap();
    if let Some(caps) = re.captures(text) {
      if let Some(m) = caps.get(1) {
        let num = m.as_str().trim();
        // Avoid matching things that are clearly not invoice numbers
        if !num.chars().all(|c| c.is_ascii_digit()) || num.len() <= 10 {
          return Some(num.to_string());
        }
      }
    }
  }

  None
}

/// Extract invoice date from text.
/// Returns date in YYYY-MM-DD format for HTML date input compatibility.
fn extract_invoice_date(text: &str) -> Option<String> {
  let date_keywords = [
    "invoice date",
    "arve kuupäev",
    "date:",
    "kuupäev:",
    "dated:",
    "issue date",
    "дата:",
    "дата счета",
  ];

  let text_lower = text.to_lowercase();

  for keyword in &date_keywords {
    if let Some(pos) = text_lower.find(keyword) {
      let after = &text[pos + keyword.len()..];
      let search_slice = &after[..100.min(after.len())];

      if let Some(date) = parse_date_from_text(search_slice) {
        return Some(date);
      }
    }
  }

  // Fallback: find any date in the first portion of the document
  let first_chunk = &text[..2000.min(text.len())];
  parse_date_from_text(first_chunk)
}

/// Try to parse a date from a text fragment.
/// Supports DD.MM.YYYY, DD/MM/YYYY, YYYY-MM-DD formats.
/// Returns YYYY-MM-DD format.
fn parse_date_from_text(text: &str) -> Option<String> {
  // DD.MM.YYYY or DD/MM/YYYY
  let re_dmy = Regex::new(r"(\d{1,2})[./](\d{1,2})[./](\d{4})").unwrap();
  if let Some(caps) = re_dmy.captures(text) {
    let day: u32 = caps[1].parse().ok()?;
    let month: u32 = caps[2].parse().ok()?;
    let year: i32 = caps[3].parse().ok()?;
    if (1..=12).contains(&month) && (1..=31).contains(&day) && (2000..=2100).contains(&year) {
      return Some(format!("{:04}-{:02}-{:02}", year, month, day));
    }
  }

  // YYYY-MM-DD
  let re_ymd = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
  if let Some(caps) = re_ymd.captures(text) {
    let year: i32 = caps[1].parse().ok()?;
    let month: u32 = caps[2].parse().ok()?;
    let day: u32 = caps[3].parse().ok()?;
    if (1..=12).contains(&month) && (1..=31).contains(&day) && (2000..=2100).contains(&year) {
      return Some(format!("{:04}-{:02}-{:02}", year, month, day));
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_amount_with_keyword() {
    let text = "Some invoice text\nTotal: 1 234,56 EUR\nThank you";
    assert_eq!(extract_amount(text), Some("1234.56".to_string()));
  }

  #[test]
  fn test_extract_amount_comma_decimal() {
    let text = "Invoice total 99,50";
    assert_eq!(extract_amount(text), Some("99.50".to_string()));
  }

  #[test]
  fn test_extract_amount_dot_decimal() {
    let text = "Amount due: 150.00 USD";
    assert_eq!(extract_amount(text), Some("150.00".to_string()));
  }

  #[test]
  fn test_extract_currency_eur_symbol() {
    assert_eq!(extract_currency("Total: 100.00 €"), Some("EUR".to_string()));
  }

  #[test]
  fn test_extract_currency_eur_text() {
    assert_eq!(
      extract_currency("Amount: 100.00 EUR"),
      Some("EUR".to_string())
    );
  }

  #[test]
  fn test_extract_currency_usd() {
    assert_eq!(extract_currency("Total: $500.00"), Some("USD".to_string()));
  }

  #[test]
  fn test_extract_currency_none() {
    assert_eq!(extract_currency("Total: 100.00"), None);
  }

  #[test]
  fn test_extract_vendor_name_from_seller() {
    let text = "Seller: Acme Corp OÜ\nAddress: Tallinn";
    assert_eq!(extract_vendor_name(text), Some("Acme Corp OÜ".to_string()));
  }

  #[test]
  fn test_extract_vendor_name_fallback() {
    let text = "TechSupplier AS\nInvoice #12345\nDate: 2026-01-15";
    assert_eq!(
      extract_vendor_name(text),
      Some("TechSupplier AS".to_string())
    );
  }

  #[test]
  fn test_extract_invoice_number() {
    let text = "Invoice Nr: INV-2026-0042\nDate: 15.01.2026";
    assert_eq!(
      extract_invoice_number(text),
      Some("INV-2026-0042".to_string())
    );
  }

  #[test]
  fn test_extract_invoice_number_hash() {
    let text = "Invoice #A1234\nTotal: 100.00";
    assert_eq!(extract_invoice_number(text), Some("A1234".to_string()));
  }

  #[test]
  fn test_extract_date_dmy() {
    let text = "Invoice date: 15.03.2026";
    assert_eq!(extract_invoice_date(text), Some("2026-03-15".to_string()));
  }

  #[test]
  fn test_extract_date_ymd() {
    let text = "Date: 2026-03-15";
    assert_eq!(extract_invoice_date(text), Some("2026-03-15".to_string()));
  }

  #[test]
  fn test_normalize_amount_spaces() {
    assert_eq!(normalize_amount("1 234,56"), "1234.56");
  }

  #[test]
  fn test_normalize_amount_dot() {
    assert_eq!(normalize_amount("1234.56"), "1234.56");
  }

  #[test]
  fn test_extractor_empty_pdf() {
    let extractor = PdfInvoiceExtractor::new();
    let result = extractor.extract(b"not a pdf");
    assert!(result.is_ok());
    let data = result.unwrap();
    assert!(data.vendor_name.is_none());
    assert!(data.amount.is_none());
  }

  #[test]
  fn test_extractor_empty_bytes() {
    let extractor = PdfInvoiceExtractor::new();
    let result = extractor.extract(b"");
    assert!(result.is_ok());
  }

  #[test]
  fn test_extracted_invoice_data_default() {
    let data = ExtractedInvoiceData::default();
    assert!(data.vendor_name.is_none());
    assert!(data.amount.is_none());
    assert!(data.currency.is_none());
    assert!(data.invoice_number.is_none());
    assert!(data.invoice_date.is_none());
  }

  #[test]
  fn test_extract_amount_estonian_keyword() {
    let text = "Arve\nKokku tasuda: 350,00 EUR";
    assert_eq!(extract_amount(text), Some("350.00".to_string()));
  }

  #[test]
  fn test_extract_amount_summa() {
    let text = "Summa kokku 1500,00";
    assert_eq!(extract_amount(text), Some("1500.00".to_string()));
  }

  #[test]
  fn test_extract_amount_fallback_largest() {
    // No keyword match — should pick the largest amount
    let text = "Line 1: 10,00\nLine 2: 50,00\nLine 3: 25,00";
    assert_eq!(extract_amount(text), Some("50.00".to_string()));
  }

  #[test]
  fn test_extract_amount_prefers_grand_total_over_subtotal() {
    // Simulates a Swedbank leasing invoice where "Kokku" appears twice:
    // first as subtotal (19 260.77), then "ARVE SUMMA KOKKU" as grand total (23 847.82)
    let text = "Kokku,\nsealhulgas\n19 260.77 4 587.05\n\
                24% maksustatav käive 19 112.69\n\
                Käibemaks (24%) 4 587.05\n\
                ARVE SUMMA KOKKU 23 847.82";
    assert_eq!(extract_amount(text), Some("23847.82".to_string()));
  }

  #[test]
  fn test_extract_amount_arve_summa_kokku() {
    let text = "Some lines\nARVE SUMMA KOKKU 1 234,56\nFooter";
    assert_eq!(extract_amount(text), Some("1234.56".to_string()));
  }

  #[test]
  fn test_extract_amount_generic_kokku_uses_last_occurrence() {
    // When only generic "kokku" appears, prefer the last one (grand total at bottom)
    let text = "Kokku 100,00\nMore lines\nKokku 500,00";
    assert_eq!(extract_amount(text), Some("500.00".to_string()));
  }

  #[test]
  fn test_extract_currency_gbp() {
    assert_eq!(extract_currency("Total: £200.00"), Some("GBP".to_string()));
  }

  #[test]
  fn test_extract_currency_sek() {
    assert_eq!(
      extract_currency("Amount: 1000.00 SEK"),
      Some("SEK".to_string())
    );
  }

  #[test]
  fn test_extract_vendor_name_from_keyword() {
    let text = "From: Baltic Imports OÜ\nInvoice #123";
    assert_eq!(
      extract_vendor_name(text),
      Some("Baltic Imports OÜ".to_string())
    );
  }

  #[test]
  fn test_extract_vendor_name_muuja() {
    let text = "Müüja: Eesti Firma AS\nOstja: My Company";
    assert_eq!(
      extract_vendor_name(text),
      Some("Eesti Firma AS".to_string())
    );
  }

  #[test]
  fn test_extract_vendor_name_skips_noise() {
    let text = "Invoice\n2026-01-15\nReal Company Name\nAddress line";
    assert_eq!(
      extract_vendor_name(text),
      Some("Real Company Name".to_string())
    );
  }

  #[test]
  fn test_extract_invoice_number_estonian() {
    let text = "Arve nr 2026-042\nKuupäev: 15.01.2026";
    assert_eq!(extract_invoice_number(text), Some("2026-042".to_string()));
  }

  #[test]
  fn test_extract_date_slash_format() {
    let text = "Date: 15/03/2026";
    assert_eq!(extract_invoice_date(text), Some("2026-03-15".to_string()));
  }

  #[test]
  fn test_extract_date_estonian_keyword() {
    let text = "Kuupäev: 01.12.2025";
    assert_eq!(extract_invoice_date(text), Some("2025-12-01".to_string()));
  }

  #[test]
  fn test_extract_date_invalid_month() {
    // Month 13 is invalid — should not match
    let text = "Date: 15.13.2026";
    assert_eq!(extract_invoice_date(text), None);
  }

  #[test]
  fn test_extract_date_invalid_year() {
    // Year 1999 is below range — should not match
    let text = "Date: 15.03.1999";
    assert_eq!(extract_invoice_date(text), None);
  }

  #[test]
  fn test_looks_like_header_noise() {
    assert!(looks_like_header_noise("Invoice"));
    assert!(looks_like_header_noise("2026-01-15"));
    assert!(looks_like_header_noise("15.03.2026"));
    assert!(!looks_like_header_noise("Acme Corp OÜ"));
    assert!(!looks_like_header_noise("Baltic Imports"));
  }

  #[test]
  fn test_clean_vendor_name() {
    assert_eq!(clean_vendor_name("  Acme Corp, "), "Acme Corp");
    assert_eq!(clean_vendor_name(": Vendor: "), "Vendor");
  }

  #[test]
  fn test_parse_date_from_text_no_match() {
    assert_eq!(parse_date_from_text("no dates here"), None);
  }

  #[test]
  fn test_pdf_invoice_extractor_default() {
    let extractor = PdfInvoiceExtractor;
    let result = extractor.extract(b"garbage");
    assert!(result.is_ok());
  }
}
