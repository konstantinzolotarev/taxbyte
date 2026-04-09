use async_trait::async_trait;
use chrono::NaiveDate;
use uuid::Uuid;

use super::{
  entities::{BankTransaction, MonthlyReport, ParsedTransaction, ReceivedInvoice},
  errors::ReportError,
};

#[async_trait]
pub trait MonthlyReportRepository: Send + Sync {
  async fn create(&self, report: MonthlyReport) -> Result<MonthlyReport, ReportError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<MonthlyReport>, ReportError>;
  async fn find_by_company_and_period(
    &self,
    company_id: Uuid,
    month: u32,
    year: i32,
  ) -> Result<Option<MonthlyReport>, ReportError>;
  async fn find_by_company_id(&self, company_id: Uuid) -> Result<Vec<MonthlyReport>, ReportError>;
  async fn update(&self, report: MonthlyReport) -> Result<MonthlyReport, ReportError>;
  async fn delete(&self, id: Uuid) -> Result<(), ReportError>;
}

#[async_trait]
pub trait BankTransactionRepository: Send + Sync {
  async fn create_many(
    &self,
    transactions: Vec<BankTransaction>,
  ) -> Result<Vec<BankTransaction>, ReportError>;
  async fn find_by_report_id(&self, report_id: Uuid) -> Result<Vec<BankTransaction>, ReportError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<BankTransaction>, ReportError>;
  async fn update_match(
    &self,
    transaction_id: Uuid,
    invoice_id: Option<Uuid>,
    received_invoice_id: Option<Uuid>,
  ) -> Result<(), ReportError>;
  async fn clear_match(&self, transaction_id: Uuid) -> Result<(), ReportError>;
  async fn update_receipt_path(
    &self,
    transaction_id: Uuid,
    receipt_path: Option<String>,
  ) -> Result<(), ReportError>;
  async fn delete_by_report_id(&self, report_id: Uuid) -> Result<(), ReportError>;
}

#[async_trait]
pub trait ReceivedInvoiceRepository: Send + Sync {
  async fn create(&self, invoice: ReceivedInvoice) -> Result<ReceivedInvoice, ReportError>;
  async fn find_by_id(&self, id: Uuid) -> Result<Option<ReceivedInvoice>, ReportError>;
  async fn find_by_company_id(&self, company_id: Uuid)
  -> Result<Vec<ReceivedInvoice>, ReportError>;
  async fn find_by_company_and_date_range(
    &self,
    company_id: Uuid,
    start_date: NaiveDate,
    end_date: NaiveDate,
  ) -> Result<Vec<ReceivedInvoice>, ReportError>;
  async fn find_unmatched_by_company(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<ReceivedInvoice>, ReportError>;
  async fn delete(&self, id: Uuid) -> Result<(), ReportError>;
}

/// Port for parsing bank statement CSV files
pub trait BankStatementParser: Send + Sync {
  fn parse(&self, csv_content: &[u8]) -> Result<Vec<ParsedTransaction>, ReportError>;
}

/// Extracted invoice data from a PDF file — all fields optional (best-effort)
#[derive(Debug, Default, Clone)]
pub struct ExtractedInvoiceData {
  pub vendor_name: Option<String>,
  pub amount: Option<String>,
  pub currency: Option<String>,
  pub invoice_number: Option<String>,
  pub invoice_date: Option<String>,
}

/// Port for extracting structured data from invoice PDFs
pub trait InvoiceDataExtractor: Send + Sync {
  fn extract(&self, pdf_bytes: &[u8]) -> Result<ExtractedInvoiceData, ReportError>;
}

/// Port for cloud storage operations specific to reports
#[async_trait]
pub trait ReportCloudStorage: Send + Sync {
  async fn create_folder(&self, parent_id: &str, name: &str) -> Result<String, ReportError>;
  async fn upload_file(
    &self,
    folder_id: &str,
    file_name: &str,
    local_path: &str,
    mime_type: &str,
  ) -> Result<String, ReportError>;
}
