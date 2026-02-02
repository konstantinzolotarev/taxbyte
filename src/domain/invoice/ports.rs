use async_trait::async_trait;
use chrono::NaiveDate;
use uuid::Uuid;

use super::entities::{
  Customer, Invoice, InvoiceLineItem, InvoiceTemplate, InvoiceTemplateLineItem,
};
use super::errors::InvoiceError;
use super::value_objects::InvoiceStatus;

#[async_trait]
pub trait CustomerRepository: Send + Sync {
  async fn create(&self, customer: Customer) -> Result<Customer, InvoiceError>;
  async fn update(&self, customer: Customer) -> Result<Customer, InvoiceError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<Customer>, InvoiceError>;
  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<Customer>, InvoiceError>;
  async fn find_active_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<Customer>, InvoiceError>;
  async fn exists_by_name(
    &self,
    company_id: Uuid,
    name: &str,
    exclude_id: Option<Uuid>,
  ) -> Result<bool, InvoiceError>;
}

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
  async fn create(&self, invoice: Invoice) -> Result<Invoice, InvoiceError>;
  async fn update(&self, invoice: Invoice) -> Result<Invoice, InvoiceError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<Invoice>, InvoiceError>;
  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<Invoice>, InvoiceError>;
  async fn find_by_company_and_status(
    &self,
    company_id: Uuid,
    status: InvoiceStatus,
  ) -> Result<Vec<Invoice>, InvoiceError>;
  async fn find_by_company_and_customer(
    &self,
    company_id: Uuid,
    customer_id: Uuid,
  ) -> Result<Vec<Invoice>, InvoiceError>;
  async fn find_overdue(
    &self,
    company_id: Uuid,
    current_date: NaiveDate,
  ) -> Result<Vec<Invoice>, InvoiceError>;
  async fn delete(&self, id: Uuid) -> Result<(), InvoiceError>;
}

#[async_trait]
pub trait InvoiceLineItemRepository: Send + Sync {
  async fn create(&self, line_item: InvoiceLineItem) -> Result<InvoiceLineItem, InvoiceError>;
  async fn create_many(
    &self,
    line_items: Vec<InvoiceLineItem>,
  ) -> Result<Vec<InvoiceLineItem>, InvoiceError>;
  async fn update(&self, line_item: InvoiceLineItem) -> Result<InvoiceLineItem, InvoiceError>;
  async fn delete(&self, id: Uuid) -> Result<(), InvoiceError>;
  async fn delete_by_invoice_id(&self, invoice_id: Uuid) -> Result<(), InvoiceError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<InvoiceLineItem>, InvoiceError>;
  async fn find_by_invoice_id(
    &self,
    invoice_id: Uuid,
  ) -> Result<Vec<InvoiceLineItem>, InvoiceError>;
}

#[async_trait]
pub trait InvoiceTemplateRepository: Send + Sync {
  async fn create(&self, template: InvoiceTemplate) -> Result<InvoiceTemplate, InvoiceError>;
  async fn update(&self, template: InvoiceTemplate) -> Result<InvoiceTemplate, InvoiceError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<InvoiceTemplate>, InvoiceError>;
  async fn find_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<InvoiceTemplate>, InvoiceError>;
  async fn find_active_by_company_id(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<InvoiceTemplate>, InvoiceError>;
  async fn exists_by_name(
    &self,
    company_id: Uuid,
    name: &str,
    exclude_id: Option<Uuid>,
  ) -> Result<bool, InvoiceError>;
}

#[async_trait]
pub trait InvoiceTemplateLineItemRepository: Send + Sync {
  async fn create_many(
    &self,
    items: Vec<InvoiceTemplateLineItem>,
  ) -> Result<Vec<InvoiceTemplateLineItem>, InvoiceError>;
  async fn find_by_template_id(
    &self,
    template_id: Uuid,
  ) -> Result<Vec<InvoiceTemplateLineItem>, InvoiceError>;
  async fn delete_by_template_id(&self, template_id: Uuid) -> Result<(), InvoiceError>;
}
