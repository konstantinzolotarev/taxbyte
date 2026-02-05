use chrono::{NaiveDate, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::company::entities::{BankAccount, Company};
use crate::domain::company::ports::{
  BankAccountRepository, CompanyMemberRepository, CompanyRepository,
};

use super::entities::{
  Customer, Invoice, InvoiceLineItem, InvoiceTemplate, InvoiceTemplateLineItem, InvoiceTotals,
};
use super::errors::InvoiceError;
use super::ports::{
  CustomerRepository, InvoiceLineItemRepository, InvoiceRepository,
  InvoiceTemplateLineItemRepository, InvoiceTemplateRepository,
};
use super::value_objects::{
  Currency, CustomerAddress, CustomerName, InvoiceNumber, InvoiceStatus, LineItemDescription,
  Money, PaymentTerms, Quantity, TemplateName, VatRate,
};

/// Invoice creation data
pub struct InvoiceData {
  pub customer_id: Uuid,
  pub bank_account_id: Option<Uuid>,
  pub invoice_number: String,
  pub invoice_date: NaiveDate,
  pub payment_terms: PaymentTerms,
  pub currency: Currency,
  pub line_items: Vec<(LineItemDescription, Quantity, Money, VatRate)>,
}

/// Invoice update data (no currency since it's from existing invoice)
pub struct InvoiceUpdateData {
  pub customer_id: Uuid,
  pub bank_account_id: Option<Uuid>,
  pub invoice_date: NaiveDate,
  pub payment_terms: PaymentTerms,
  pub line_items: Vec<(LineItemDescription, Quantity, Money, VatRate)>,
}

/// Dependencies for InvoiceService
pub struct InvoiceServiceDependencies {
  pub invoice_repo: Arc<dyn InvoiceRepository>,
  pub line_item_repo: Arc<dyn InvoiceLineItemRepository>,
  pub customer_repo: Arc<dyn CustomerRepository>,
  pub company_member_repo: Arc<dyn CompanyMemberRepository>,
  pub company_repo: Arc<dyn CompanyRepository>,
  pub bank_account_repo: Arc<dyn BankAccountRepository>,
  pub template_repo: Arc<dyn InvoiceTemplateRepository>,
  pub template_line_item_repo: Arc<dyn InvoiceTemplateLineItemRepository>,
}

pub struct InvoiceService {
  invoice_repo: Arc<dyn InvoiceRepository>,
  line_item_repo: Arc<dyn InvoiceLineItemRepository>,
  customer_repo: Arc<dyn CustomerRepository>,
  company_member_repo: Arc<dyn CompanyMemberRepository>,
  company_repo: Arc<dyn CompanyRepository>,
  bank_account_repo: Arc<dyn BankAccountRepository>,
  template_repo: Arc<dyn InvoiceTemplateRepository>,
  template_line_item_repo: Arc<dyn InvoiceTemplateLineItemRepository>,
}

impl InvoiceService {
  pub fn new(deps: InvoiceServiceDependencies) -> Self {
    Self {
      invoice_repo: deps.invoice_repo,
      line_item_repo: deps.line_item_repo,
      customer_repo: deps.customer_repo,
      company_member_repo: deps.company_member_repo,
      company_repo: deps.company_repo,
      bank_account_repo: deps.bank_account_repo,
      template_repo: deps.template_repo,
      template_line_item_repo: deps.template_line_item_repo,
    }
  }

  // Customer operations
  pub async fn create_customer(
    &self,
    user_id: Uuid,
    company_id: Uuid,
    name: CustomerName,
    address: Option<CustomerAddress>,
  ) -> Result<Customer, InvoiceError> {
    // Verify user is company member
    self.verify_company_membership(user_id, company_id).await?;

    // Check for duplicate name
    if self
      .customer_repo
      .exists_by_name(company_id, name.value(), None)
      .await?
    {
      return Err(InvoiceError::CustomerNameAlreadyExists);
    }

    let customer = Customer::new(company_id, name, address);
    self.customer_repo.create(customer).await
  }

  pub async fn update_customer(
    &self,
    user_id: Uuid,
    customer_id: Uuid,
    name: CustomerName,
    address: Option<CustomerAddress>,
  ) -> Result<Customer, InvoiceError> {
    let mut customer = self
      .customer_repo
      .find_by_id(customer_id)
      .await?
      .ok_or(InvoiceError::CustomerNotFound(customer_id))?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, customer.company_id)
      .await?;

    // Check for duplicate name (excluding current customer)
    if self
      .customer_repo
      .exists_by_name(customer.company_id, name.value(), Some(customer_id))
      .await?
    {
      return Err(InvoiceError::CustomerNameAlreadyExists);
    }

    customer.update(name, address);
    self.customer_repo.update(customer).await
  }

  pub async fn archive_customer(
    &self,
    user_id: Uuid,
    customer_id: Uuid,
  ) -> Result<(), InvoiceError> {
    let mut customer = self
      .customer_repo
      .find_by_id(customer_id)
      .await?
      .ok_or(InvoiceError::CustomerNotFound(customer_id))?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, customer.company_id)
      .await?;

    customer.archive();
    self.customer_repo.update(customer).await?;
    Ok(())
  }

  pub async fn get_customer(
    &self,
    user_id: Uuid,
    customer_id: Uuid,
  ) -> Result<Customer, InvoiceError> {
    let customer = self
      .customer_repo
      .find_by_id(customer_id)
      .await?
      .ok_or(InvoiceError::CustomerNotFound(customer_id))?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, customer.company_id)
      .await?;

    Ok(customer)
  }

  pub async fn list_customers(
    &self,
    user_id: Uuid,
    company_id: Uuid,
    include_archived: bool,
  ) -> Result<Vec<Customer>, InvoiceError> {
    // Verify user is company member
    self.verify_company_membership(user_id, company_id).await?;

    if include_archived {
      self.customer_repo.find_by_company_id(company_id).await
    } else {
      self
        .customer_repo
        .find_active_by_company_id(company_id)
        .await
    }
  }

  // Invoice operations
  pub async fn create_invoice(
    &self,
    user_id: Uuid,
    company_id: Uuid,
    data: InvoiceData,
  ) -> Result<(Invoice, Vec<InvoiceLineItem>), InvoiceError> {
    // Verify user is company member
    self.verify_company_membership(user_id, company_id).await?;

    // Verify customer exists and belongs to company
    let customer = self
      .customer_repo
      .find_by_id(data.customer_id)
      .await?
      .ok_or(InvoiceError::CustomerNotFound(data.customer_id))?;

    if customer.company_id != company_id {
      return Err(InvoiceError::PermissionDenied(
        "Customer does not belong to this company".to_string(),
      ));
    }

    // Validate line items
    if data.line_items.is_empty() {
      return Err(InvoiceError::NoLineItems);
    }

    // Verify all line items have the same currency
    for (_, _, unit_price, _) in &data.line_items {
      if unit_price.currency != data.currency {
        return Err(InvoiceError::CurrencyMismatch {
          expected: data.currency.as_str().to_string(),
          actual: unit_price.currency.as_str().to_string(),
        });
      }
    }

    // Validate and create invoice number
    let invoice_number = InvoiceNumber::new(data.invoice_number)?;

    // Create invoice
    let invoice = Invoice::new(
      company_id,
      data.customer_id,
      data.bank_account_id,
      invoice_number,
      data.invoice_date,
      data.payment_terms,
      data.currency,
    );

    let created_invoice = self.invoice_repo.create(invoice).await?;

    // Create line items
    let line_items_entities: Vec<InvoiceLineItem> = data
      .line_items
      .into_iter()
      .enumerate()
      .map(|(i, (description, quantity, unit_price, vat_rate))| {
        InvoiceLineItem::new(
          created_invoice.id,
          description,
          quantity,
          unit_price,
          vat_rate,
          (i + 1) as i32,
        )
      })
      .collect();

    let created_line_items = self.line_item_repo.create_many(line_items_entities).await?;

    Ok((created_invoice, created_line_items))
  }

  pub async fn update_invoice(
    &self,
    user_id: Uuid,
    invoice_id: Uuid,
    data: InvoiceUpdateData,
  ) -> Result<(Invoice, Vec<InvoiceLineItem>), InvoiceError> {
    let mut invoice = self
      .invoice_repo
      .find_by_id(invoice_id)
      .await?
      .ok_or(InvoiceError::InvoiceNotFound(invoice_id))?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, invoice.company_id)
      .await?;

    // Verify invoice is editable
    if !invoice.is_editable() {
      return Err(InvoiceError::CannotEditInvoice(
        "Invoice is not in draft status".to_string(),
      ));
    }

    // Verify customer exists and belongs to company
    let customer = self
      .customer_repo
      .find_by_id(data.customer_id)
      .await?
      .ok_or(InvoiceError::CustomerNotFound(data.customer_id))?;

    if customer.company_id != invoice.company_id {
      return Err(InvoiceError::PermissionDenied(
        "Customer does not belong to this company".to_string(),
      ));
    }

    // Validate line items
    if data.line_items.is_empty() {
      return Err(InvoiceError::NoLineItems);
    }

    // Verify all line items have the same currency
    for (_, _, unit_price, _) in &data.line_items {
      if unit_price.currency != invoice.currency {
        return Err(InvoiceError::CurrencyMismatch {
          expected: invoice.currency.as_str().to_string(),
          actual: unit_price.currency.as_str().to_string(),
        });
      }
    }

    // Update invoice
    invoice.update(
      data.customer_id,
      data.bank_account_id,
      data.invoice_date,
      data.payment_terms,
    )?;

    let updated_invoice = self.invoice_repo.update(invoice).await?;

    // Delete old line items and create new ones
    self.line_item_repo.delete_by_invoice_id(invoice_id).await?;

    let line_items_entities: Vec<InvoiceLineItem> = data
      .line_items
      .into_iter()
      .enumerate()
      .map(|(i, (description, quantity, unit_price, vat_rate))| {
        InvoiceLineItem::new(
          updated_invoice.id,
          description,
          quantity,
          unit_price,
          vat_rate,
          (i + 1) as i32,
        )
      })
      .collect();

    let created_line_items = self.line_item_repo.create_many(line_items_entities).await?;

    Ok((updated_invoice, created_line_items))
  }

  pub async fn change_invoice_status(
    &self,
    user_id: Uuid,
    invoice_id: Uuid,
    new_status: InvoiceStatus,
  ) -> Result<Invoice, InvoiceError> {
    let mut invoice = self
      .invoice_repo
      .find_by_id(invoice_id)
      .await?
      .ok_or(InvoiceError::InvoiceNotFound(invoice_id))?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, invoice.company_id)
      .await?;

    invoice.change_status(new_status)?;

    self.invoice_repo.update(invoice).await
  }

  pub async fn archive_invoice(&self, user_id: Uuid, invoice_id: Uuid) -> Result<(), InvoiceError> {
    let mut invoice = self
      .invoice_repo
      .find_by_id(invoice_id)
      .await?
      .ok_or(InvoiceError::InvoiceNotFound(invoice_id))?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, invoice.company_id)
      .await?;

    invoice.archive();
    self.invoice_repo.update(invoice).await?;
    Ok(())
  }

  pub async fn delete_invoice(&self, user_id: Uuid, invoice_id: Uuid) -> Result<(), InvoiceError> {
    let invoice = self
      .invoice_repo
      .find_by_id(invoice_id)
      .await?
      .ok_or(InvoiceError::InvoiceNotFound(invoice_id))?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, invoice.company_id)
      .await?;

    // Only allow deleting draft invoices
    if invoice.status != InvoiceStatus::Draft {
      return Err(InvoiceError::CannotDeleteInvoice(
        "Only draft invoices can be deleted. Use archive for other statuses.".to_string(),
      ));
    }

    // Delete the invoice (line items will be deleted by the repository via CASCADE)
    self.invoice_repo.delete(invoice_id).await?;
    Ok(())
  }

  pub async fn get_invoice(
    &self,
    user_id: Uuid,
    invoice_id: Uuid,
  ) -> Result<Invoice, InvoiceError> {
    let invoice = self
      .invoice_repo
      .find_by_id(invoice_id)
      .await?
      .ok_or(InvoiceError::InvoiceNotFound(invoice_id))?;

    // Verify user is company member (skip for system user - nil UUID for PDF generation)
    // SECURITY: Nil UUID bypass is safe because the /invoices/{id}/html endpoint
    // is protected by IP whitelist (localhost only). See invoice_html_view handler.
    if !user_id.is_nil() {
      self
        .verify_company_membership(user_id, invoice.company_id)
        .await?;
    }

    Ok(invoice)
  }

  pub async fn get_invoice_with_details(
    &self,
    user_id: Uuid,
    invoice_id: Uuid,
  ) -> Result<
    (
      Invoice,
      Vec<InvoiceLineItem>,
      Customer,
      Company,
      Option<BankAccount>,
      InvoiceTotals,
    ),
    InvoiceError,
  > {
    let invoice = self.get_invoice(user_id, invoice_id).await?;

    let line_items = self.line_item_repo.find_by_invoice_id(invoice_id).await?;

    let customer = self
      .customer_repo
      .find_by_id(invoice.customer_id)
      .await?
      .ok_or(InvoiceError::CustomerNotFound(invoice.customer_id))?;

    let company = self
      .company_repo
      .find_by_id(invoice.company_id)
      .await
      .map_err(|e| InvoiceError::Internal(format!("Failed to fetch company: {}", e)))?
      .ok_or_else(|| InvoiceError::Internal(format!("Company {} not found", invoice.company_id)))?;

    // Conditionally fetch bank account if referenced
    let bank_account = if let Some(bank_account_id) = invoice.bank_account_id {
      self
        .bank_account_repo
        .find_by_id(bank_account_id)
        .await
        .map_err(|e| InvoiceError::Internal(format!("Failed to fetch bank account: {}", e)))?
    } else {
      None
    };

    let totals = InvoiceTotals::calculate(&line_items, invoice.currency);

    Ok((invoice, line_items, customer, company, bank_account, totals))
  }

  pub async fn list_invoices(
    &self,
    user_id: Uuid,
    company_id: Uuid,
    status_filter: Option<InvoiceStatus>,
    customer_filter: Option<Uuid>,
  ) -> Result<Vec<Invoice>, InvoiceError> {
    // Verify user is company member
    self.verify_company_membership(user_id, company_id).await?;

    if let Some(status) = status_filter {
      self
        .invoice_repo
        .find_by_company_and_status(company_id, status)
        .await
    } else if let Some(customer_id) = customer_filter {
      self
        .invoice_repo
        .find_by_company_and_customer(company_id, customer_id)
        .await
    } else {
      self.invoice_repo.find_by_company_id(company_id).await
    }
  }

  pub async fn set_invoice_pdf_path(
    &self,
    invoice_id: Uuid,
    pdf_path: String,
    drive_file_id: Option<String>,
  ) -> Result<Invoice, InvoiceError> {
    let mut invoice = self
      .invoice_repo
      .find_by_id(invoice_id)
      .await?
      .ok_or(InvoiceError::InvoiceNotFound(invoice_id))?;

    invoice.set_pdf_path(pdf_path);
    invoice.pdf_drive_file_id = drive_file_id;
    self.invoice_repo.update(invoice).await
  }

  pub async fn mark_overdue_invoices(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<Invoice>, InvoiceError> {
    let current_date = Utc::now().date_naive();
    let overdue_invoices = self
      .invoice_repo
      .find_overdue(company_id, current_date)
      .await?;

    let mut updated_invoices = Vec::new();
    for mut invoice in overdue_invoices {
      if invoice.is_overdue(current_date) {
        invoice.change_status(InvoiceStatus::Overdue)?;
        let updated = self.invoice_repo.update(invoice).await?;
        updated_invoices.push(updated);
      }
    }

    Ok(updated_invoices)
  }

  // Template operations
  pub async fn create_template_from_invoice(
    &self,
    user_id: Uuid,
    invoice_id: Uuid,
    template_name: TemplateName,
    description: Option<String>,
  ) -> Result<(InvoiceTemplate, Vec<InvoiceTemplateLineItem>), InvoiceError> {
    // Fetch invoice with line items
    let invoice = self
      .invoice_repo
      .find_by_id(invoice_id)
      .await?
      .ok_or(InvoiceError::InvoiceNotFound(invoice_id))?;

    let line_items = self.line_item_repo.find_by_invoice_id(invoice_id).await?;

    // Verify user is company member
    self
      .verify_company_membership(user_id, invoice.company_id)
      .await?;

    // Check for duplicate template name
    if self
      .template_repo
      .exists_by_name(invoice.company_id, template_name.value(), None)
      .await?
    {
      return Err(InvoiceError::TemplateNameAlreadyExists(
        template_name.into_inner(),
      ));
    }

    // Create template from invoice data
    let template = InvoiceTemplate::new(
      invoice.company_id,
      template_name,
      description,
      invoice.customer_id,
      invoice.bank_account_id,
      invoice.payment_terms,
      invoice.currency,
    );

    let created_template = self.template_repo.create(template).await?;

    // Create template line items from invoice line items
    let template_items: Vec<InvoiceTemplateLineItem> = line_items
      .into_iter()
      .map(|item| {
        InvoiceTemplateLineItem::new(
          created_template.id,
          item.description,
          item.quantity,
          item.unit_price,
          item.vat_rate,
          item.line_order,
        )
      })
      .collect();

    let created_items = self
      .template_line_item_repo
      .create_many(template_items)
      .await?;

    Ok((created_template, created_items))
  }

  pub async fn list_templates(
    &self,
    user_id: Uuid,
    company_id: Uuid,
    include_archived: bool,
  ) -> Result<Vec<InvoiceTemplate>, InvoiceError> {
    self.verify_company_membership(user_id, company_id).await?;

    if include_archived {
      self.template_repo.find_by_company_id(company_id).await
    } else {
      self
        .template_repo
        .find_active_by_company_id(company_id)
        .await
    }
  }

  pub async fn get_template_with_items(
    &self,
    user_id: Uuid,
    template_id: Uuid,
  ) -> Result<(InvoiceTemplate, Vec<InvoiceTemplateLineItem>), InvoiceError> {
    let template = self
      .template_repo
      .find_by_id(template_id)
      .await?
      .ok_or(InvoiceError::TemplateNotFound(template_id))?;

    self
      .verify_company_membership(user_id, template.company_id)
      .await?;

    let items = self
      .template_line_item_repo
      .find_by_template_id(template_id)
      .await?;

    Ok((template, items))
  }

  pub async fn archive_template(
    &self,
    user_id: Uuid,
    template_id: Uuid,
  ) -> Result<(), InvoiceError> {
    let mut template = self
      .template_repo
      .find_by_id(template_id)
      .await?
      .ok_or(InvoiceError::TemplateNotFound(template_id))?;

    self
      .verify_company_membership(user_id, template.company_id)
      .await?;

    template.archive();
    self.template_repo.update(template).await?;
    Ok(())
  }

  // Helper methods
  async fn verify_company_membership(
    &self,
    user_id: Uuid,
    company_id: Uuid,
  ) -> Result<(), InvoiceError> {
    let member = self
      .company_member_repo
      .find_member(company_id, user_id)
      .await
      .map_err(|e| InvoiceError::Internal(format!("Failed to verify membership: {}", e)))?;

    if member.is_none() {
      return Err(InvoiceError::PermissionDenied(
        "User is not a member of this company".to_string(),
      ));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  // Tests would require mocking repositories, which is beyond scope of this implementation
  // In production, consider using mockall crate for repository mocks
}
