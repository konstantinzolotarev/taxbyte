use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::InvoiceEntityError;
use super::value_objects::{
  Currency, CustomerAddress, CustomerName, InvoiceNumber, InvoiceStatus, LineItemDescription,
  Money, PaymentTerms, Quantity, TemplateName, VatRate,
};

// Customer - Reusable client information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Customer {
  pub id: Uuid,
  pub company_id: Uuid,
  pub name: CustomerName,
  pub address: Option<CustomerAddress>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub archived_at: Option<DateTime<Utc>>,
}

impl Customer {
  pub fn new(company_id: Uuid, name: CustomerName, address: Option<CustomerAddress>) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4(),
      company_id,
      name,
      address,
      created_at: now,
      updated_at: now,
      archived_at: None,
    }
  }

  pub fn update(&mut self, name: CustomerName, address: Option<CustomerAddress>) {
    self.name = name;
    self.address = address;
    self.updated_at = Utc::now();
  }

  pub fn archive(&mut self) {
    self.archived_at = Some(Utc::now());
  }

  pub fn is_archived(&self) -> bool {
    self.archived_at.is_some()
  }
}

// Invoice - Main invoice document
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invoice {
  pub id: Uuid,
  pub company_id: Uuid,
  pub customer_id: Uuid,
  pub bank_account_id: Option<Uuid>,
  pub invoice_number: InvoiceNumber,
  pub invoice_date: NaiveDate,
  pub due_date: NaiveDate,
  pub payment_terms: PaymentTerms,
  pub currency: Currency,
  pub status: InvoiceStatus,
  pub pdf_path: Option<String>,
  pub pdf_drive_file_id: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub archived_at: Option<DateTime<Utc>>,
}

impl Invoice {
  pub fn new(
    company_id: Uuid,
    customer_id: Uuid,
    bank_account_id: Option<Uuid>,
    invoice_number: InvoiceNumber,
    invoice_date: NaiveDate,
    payment_terms: PaymentTerms,
    currency: Currency,
  ) -> Self {
    let now = Utc::now();
    let due_date = invoice_date + chrono::Duration::days(payment_terms.days() as i64);

    Self {
      id: Uuid::new_v4(),
      company_id,
      customer_id,
      bank_account_id,
      invoice_number,
      invoice_date,
      due_date,
      payment_terms,
      currency,
      status: InvoiceStatus::Draft,
      pdf_path: None,
      pdf_drive_file_id: None,
      created_at: now,
      updated_at: now,
      archived_at: None,
    }
  }

  pub fn update(
    &mut self,
    customer_id: Uuid,
    bank_account_id: Option<Uuid>,
    invoice_date: NaiveDate,
    payment_terms: PaymentTerms,
  ) -> Result<(), InvoiceEntityError> {
    if !self.status.is_editable() {
      return Err(InvoiceEntityError::CannotEditNonDraftInvoice {
        current_status: self.status,
      });
    }

    self.customer_id = customer_id;
    self.bank_account_id = bank_account_id;
    self.invoice_date = invoice_date;
    self.payment_terms = payment_terms;
    self.due_date = invoice_date + chrono::Duration::days(payment_terms.days() as i64);
    self.updated_at = Utc::now();

    Ok(())
  }

  pub fn change_status(&mut self, new_status: InvoiceStatus) -> Result<(), InvoiceEntityError> {
    if !self.status.can_transition_to(new_status) {
      return Err(InvoiceEntityError::InvalidStatusTransition {
        from: self.status,
        to: new_status,
      });
    }

    self.status = new_status;
    self.updated_at = Utc::now();
    Ok(())
  }

  pub fn set_pdf_path(&mut self, path: String) {
    self.pdf_path = Some(path);
    self.updated_at = Utc::now();
  }

  pub fn archive(&mut self) {
    self.archived_at = Some(Utc::now());
  }

  pub fn is_archived(&self) -> bool {
    self.archived_at.is_some()
  }

  pub fn is_editable(&self) -> bool {
    self.status.is_editable()
  }

  pub fn is_overdue(&self, current_date: NaiveDate) -> bool {
    self.status == InvoiceStatus::Sent && self.due_date < current_date
  }
}

// Invoice Line Item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceLineItem {
  pub id: Uuid,
  pub invoice_id: Uuid,
  pub description: LineItemDescription,
  pub quantity: Quantity,
  pub unit_price: Money,
  pub vat_rate: VatRate,
  pub line_order: i32,
}

impl InvoiceLineItem {
  pub fn new(
    invoice_id: Uuid,
    description: LineItemDescription,
    quantity: Quantity,
    unit_price: Money,
    vat_rate: VatRate,
    line_order: i32,
  ) -> Self {
    Self {
      id: Uuid::new_v4(),
      invoice_id,
      description,
      quantity,
      unit_price,
      vat_rate,
      line_order,
    }
  }

  pub fn subtotal(&self) -> Money {
    self.unit_price.multiply(self.quantity.value())
  }

  pub fn vat_amount(&self) -> Money {
    self.subtotal().multiply(self.vat_rate.as_multiplier())
  }

  pub fn total(&self) -> Money {
    let subtotal = self.subtotal();
    let vat = self.vat_amount();
    subtotal
      .add(&vat)
      .expect("Currency mismatch in line item total")
  }
}

// Invoice Totals - Calculated, not persisted
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceTotals {
  pub subtotal: Money,
  pub total_vat: Money,
  pub grand_total: Money,
}

impl InvoiceTotals {
  pub fn calculate(line_items: &[InvoiceLineItem], currency: Currency) -> Self {
    let subtotal = line_items.iter().fold(Money::zero(currency), |acc, item| {
      acc.add(&item.subtotal()).expect("Currency mismatch")
    });

    let total_vat = line_items.iter().fold(Money::zero(currency), |acc, item| {
      acc.add(&item.vat_amount()).expect("Currency mismatch")
    });

    let grand_total = subtotal.add(&total_vat).expect("Currency mismatch");

    Self {
      subtotal,
      total_vat,
      grand_total,
    }
  }
}

// Invoice Template - Reusable invoice configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceTemplate {
  pub id: Uuid,
  pub company_id: Uuid,
  pub name: TemplateName,
  pub description: Option<String>,
  pub customer_id: Uuid,
  pub bank_account_id: Option<Uuid>,
  pub payment_terms: PaymentTerms,
  pub currency: Currency,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
  pub archived_at: Option<DateTime<Utc>>,
}

impl InvoiceTemplate {
  pub fn new(
    company_id: Uuid,
    name: TemplateName,
    description: Option<String>,
    customer_id: Uuid,
    bank_account_id: Option<Uuid>,
    payment_terms: PaymentTerms,
    currency: Currency,
  ) -> Self {
    let now = Utc::now();
    Self {
      id: Uuid::new_v4(),
      company_id,
      name,
      description,
      customer_id,
      bank_account_id,
      payment_terms,
      currency,
      created_at: now,
      updated_at: now,
      archived_at: None,
    }
  }

  pub fn archive(&mut self) {
    self.archived_at = Some(Utc::now());
    self.updated_at = Utc::now();
  }

  pub fn is_archived(&self) -> bool {
    self.archived_at.is_some()
  }
}

// Invoice Template Line Item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceTemplateLineItem {
  pub id: Uuid,
  pub template_id: Uuid,
  pub description: LineItemDescription,
  pub quantity: Quantity,
  pub unit_price: Money,
  pub vat_rate: VatRate,
  pub line_order: i32,
}

impl InvoiceTemplateLineItem {
  pub fn new(
    template_id: Uuid,
    description: LineItemDescription,
    quantity: Quantity,
    unit_price: Money,
    vat_rate: VatRate,
    line_order: i32,
  ) -> Self {
    Self {
      id: Uuid::new_v4(),
      template_id,
      description,
      quantity,
      unit_price,
      vat_rate,
      line_order,
    }
  }

  pub fn subtotal(&self) -> Money {
    self.unit_price.multiply(self.quantity.value())
  }

  pub fn vat_amount(&self) -> Money {
    self.subtotal().multiply(self.vat_rate.as_multiplier())
  }

  pub fn total(&self) -> Money {
    let subtotal = self.subtotal();
    let vat = self.vat_amount();
    subtotal
      .add(&vat)
      .expect("Currency mismatch in template line item total")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use rust_decimal_macros::dec;

  #[test]
  fn test_customer_creation() {
    let name = CustomerName::new("Test Customer".to_string()).unwrap();
    let customer = Customer::new(Uuid::new_v4(), name, None);
    assert!(!customer.is_archived());
  }

  #[test]
  fn test_customer_archive() {
    let name = CustomerName::new("Test Customer".to_string()).unwrap();
    let mut customer = Customer::new(Uuid::new_v4(), name, None);
    customer.archive();
    assert!(customer.is_archived());
  }

  #[test]
  fn test_invoice_creation() {
    let invoice = Invoice::new(
      Uuid::new_v4(),
      Uuid::new_v4(),
      None,
      InvoiceNumber::new("INV-001".to_string()).unwrap(),
      NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
      PaymentTerms::Net30,
      Currency::USD,
    );

    assert_eq!(invoice.status, InvoiceStatus::Draft);
    assert_eq!(
      invoice.due_date,
      NaiveDate::from_ymd_opt(2026, 3, 3).unwrap()
    );
    assert!(invoice.is_editable());
  }

  #[test]
  fn test_invoice_status_change() {
    let mut invoice = Invoice::new(
      Uuid::new_v4(),
      Uuid::new_v4(),
      None,
      InvoiceNumber::new("INV-001".to_string()).unwrap(),
      NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
      PaymentTerms::Net30,
      Currency::USD,
    );

    assert!(invoice.change_status(InvoiceStatus::Sent).is_ok());
    assert_eq!(invoice.status, InvoiceStatus::Sent);
    assert!(!invoice.is_editable());

    assert!(invoice.change_status(InvoiceStatus::Draft).is_err());
    assert!(invoice.change_status(InvoiceStatus::Paid).is_ok());
  }

  #[test]
  fn test_invoice_update_only_when_draft() {
    let mut invoice = Invoice::new(
      Uuid::new_v4(),
      Uuid::new_v4(),
      None,
      InvoiceNumber::new("INV-001".to_string()).unwrap(),
      NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
      PaymentTerms::Net30,
      Currency::USD,
    );

    // Should work when draft
    assert!(
      invoice
        .update(
          Uuid::new_v4(),
          None,
          NaiveDate::from_ymd_opt(2026, 2, 2).unwrap(),
          PaymentTerms::Net15
        )
        .is_ok()
    );

    // Should fail when sent
    invoice.change_status(InvoiceStatus::Sent).unwrap();
    assert!(
      invoice
        .update(
          Uuid::new_v4(),
          None,
          NaiveDate::from_ymd_opt(2026, 2, 3).unwrap(),
          PaymentTerms::Net15
        )
        .is_err()
    );
  }

  #[test]
  fn test_line_item_calculations() {
    let line_item = InvoiceLineItem::new(
      Uuid::new_v4(),
      LineItemDescription::new("Test Item".to_string()).unwrap(),
      Quantity::new(dec!(2)).unwrap(),
      Money::new(dec!(100), Currency::USD).unwrap(),
      VatRate::new(dec!(25)).unwrap(),
      1,
    );

    assert_eq!(line_item.subtotal().amount, dec!(200)); // 2 * 100
    assert_eq!(line_item.vat_amount().amount, dec!(50)); // 200 * 0.25
    assert_eq!(line_item.total().amount, dec!(250)); // 200 + 50
  }

  #[test]
  fn test_invoice_totals() {
    let line_items = vec![
      InvoiceLineItem::new(
        Uuid::new_v4(),
        LineItemDescription::new("Item 1".to_string()).unwrap(),
        Quantity::new(dec!(2)).unwrap(),
        Money::new(dec!(100), Currency::USD).unwrap(),
        VatRate::new(dec!(25)).unwrap(),
        1,
      ),
      InvoiceLineItem::new(
        Uuid::new_v4(),
        LineItemDescription::new("Item 2".to_string()).unwrap(),
        Quantity::new(dec!(1)).unwrap(),
        Money::new(dec!(50), Currency::USD).unwrap(),
        VatRate::new(dec!(25)).unwrap(),
        2,
      ),
    ];

    let totals = InvoiceTotals::calculate(&line_items, Currency::USD);
    assert_eq!(totals.subtotal.amount, dec!(250)); // 200 + 50
    assert_eq!(totals.total_vat.amount, dec!(62.5)); // 50 + 12.5
    assert_eq!(totals.grand_total.amount, dec!(312.5)); // 250 + 62.5
  }

  #[test]
  fn test_invoice_overdue() {
    let invoice = Invoice::new(
      Uuid::new_v4(),
      Uuid::new_v4(),
      None,
      InvoiceNumber::new("INV-001".to_string()).unwrap(),
      NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
      PaymentTerms::Net30,
      Currency::USD,
    );

    let current_date = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    assert!(!invoice.is_overdue(current_date)); // Draft is never overdue

    let mut invoice = invoice;
    invoice.change_status(InvoiceStatus::Sent).unwrap();
    assert!(invoice.is_overdue(current_date)); // Past due date
  }
}
