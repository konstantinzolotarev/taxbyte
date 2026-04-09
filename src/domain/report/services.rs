use std::sync::Arc;

use chrono::Utc;
use rust_decimal::Decimal;
use uuid::Uuid;

use super::{
  entities::{BankTransaction, MonthlyReport, ParsedTransaction, ReceivedInvoice},
  errors::ReportError,
  ports::{BankTransactionRepository, MonthlyReportRepository, ReceivedInvoiceRepository},
  value_objects::{ReportMonth, ReportStatus, TransactionDirection},
};

pub struct ReportService {
  report_repo: Arc<dyn MonthlyReportRepository>,
  transaction_repo: Arc<dyn BankTransactionRepository>,
  received_invoice_repo: Arc<dyn ReceivedInvoiceRepository>,
}

impl ReportService {
  pub fn new(
    report_repo: Arc<dyn MonthlyReportRepository>,
    transaction_repo: Arc<dyn BankTransactionRepository>,
    received_invoice_repo: Arc<dyn ReceivedInvoiceRepository>,
  ) -> Self {
    Self {
      report_repo,
      transaction_repo,
      received_invoice_repo,
    }
  }

  /// Create an empty report shell (no bank statement yet)
  pub async fn create_empty_report(
    &self,
    company_id: Uuid,
    period: ReportMonth,
  ) -> Result<MonthlyReport, ReportError> {
    // Check for duplicate
    if self
      .report_repo
      .find_by_company_and_period(company_id, period.month, period.year)
      .await?
      .is_some()
    {
      return Err(ReportError::DuplicateReport);
    }

    let report = MonthlyReport::new(company_id, period.month, period.year, None);
    self.report_repo.create(report).await
  }

  /// Import a bank statement: create report + transactions (or populate existing empty report)
  pub async fn import_bank_statement(
    &self,
    company_id: Uuid,
    period: ReportMonth,
    transactions: Vec<ParsedTransaction>,
  ) -> Result<MonthlyReport, ReportError> {
    // Extract IBAN from first transaction
    let iban = transactions
      .first()
      .map(|t| t.client_account.clone())
      .unwrap_or_default();

    // Calculate totals
    let mut total_incoming = Decimal::ZERO;
    let mut total_outgoing = Decimal::ZERO;
    for t in &transactions {
      match t.direction {
        TransactionDirection::Credit => total_incoming += t.amount,
        TransactionDirection::Debit => total_outgoing += t.amount.abs(),
      }
    }

    let tx_count = transactions.len() as i32;

    // Check for existing report
    let report = if let Some(existing) = self
      .report_repo
      .find_by_company_and_period(company_id, period.month, period.year)
      .await?
    {
      // Only allow populating empty draft reports
      if existing.status != ReportStatus::Draft || existing.transaction_count != 0 {
        return Err(ReportError::DuplicateReport);
      }

      let mut report = existing;
      report.bank_account_iban = Some(iban);
      report.total_incoming = total_incoming;
      report.total_outgoing = total_outgoing;
      report.transaction_count = tx_count;
      report.updated_at = Utc::now();
      self.report_repo.update(report).await?
    } else {
      let mut report = MonthlyReport::new(company_id, period.month, period.year, Some(iban));
      report.total_incoming = total_incoming;
      report.total_outgoing = total_outgoing;
      report.transaction_count = tx_count;
      self.report_repo.create(report).await?
    };

    // Create bank transactions
    let bank_transactions: Vec<BankTransaction> = transactions
      .into_iter()
      .map(|t| {
        BankTransaction::new(
          report.id,
          t.row_number,
          t.date,
          t.counterparty_name,
          t.counterparty_account,
          t.direction,
          t.amount,
          t.reference_number,
          t.description,
          t.currency,
          t.registry_code,
        )
      })
      .collect();

    let bank_transactions = self.transaction_repo.create_many(bank_transactions).await?;

    // Auto-match received invoices to debit transactions
    let auto_matched = self
      .auto_match_received_invoices(report.id, company_id, &bank_transactions)
      .await?;

    // Update matched count if any auto-matches were made
    if auto_matched > 0 {
      let mut report = self
        .report_repo
        .find_by_id(report.id)
        .await?
        .ok_or(ReportError::NotFound)?;
      report.matched_count = auto_matched;
      report.updated_at = Utc::now();
      return self.report_repo.update(report).await;
    }

    Ok(report)
  }

  /// Get all reports for a company
  pub async fn get_company_reports(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<MonthlyReport>, ReportError> {
    self.report_repo.find_by_company_id(company_id).await
  }

  /// Get report with transactions
  pub async fn get_report_details(
    &self,
    report_id: Uuid,
  ) -> Result<(MonthlyReport, Vec<BankTransaction>), ReportError> {
    let report = self
      .report_repo
      .find_by_id(report_id)
      .await?
      .ok_or(ReportError::NotFound)?;

    let transactions = self.transaction_repo.find_by_report_id(report_id).await?;

    Ok((report, transactions))
  }

  /// Match a transaction to an invoice (issued or received)
  pub async fn match_transaction(
    &self,
    transaction_id: Uuid,
    invoice_id: Option<Uuid>,
    received_invoice_id: Option<Uuid>,
  ) -> Result<(), ReportError> {
    let tx = self
      .transaction_repo
      .find_by_id(transaction_id)
      .await?
      .ok_or(ReportError::TransactionNotFound)?;

    if tx.is_matched() {
      return Err(ReportError::AlreadyMatched);
    }

    // Validate that exactly one is provided
    match (&invoice_id, &received_invoice_id) {
      (Some(_), None) | (None, Some(_)) => {}
      _ => {
        return Err(ReportError::Validation(
          "Exactly one of invoice_id or received_invoice_id must be provided".to_string(),
        ));
      }
    }

    self
      .transaction_repo
      .update_match(transaction_id, invoice_id, received_invoice_id)
      .await?;

    // Update matched count on report
    self.update_matched_count(tx.report_id).await?;

    Ok(())
  }

  /// Clear a match from a transaction
  pub async fn unmatch_transaction(&self, transaction_id: Uuid) -> Result<(), ReportError> {
    let tx = self
      .transaction_repo
      .find_by_id(transaction_id)
      .await?
      .ok_or(ReportError::TransactionNotFound)?;

    if !tx.is_matched() {
      return Err(ReportError::NotMatched);
    }

    self.transaction_repo.clear_match(transaction_id).await?;

    // Update matched count on report
    self.update_matched_count(tx.report_id).await?;

    Ok(())
  }

  /// Delete a report and its transactions
  pub async fn delete_report(&self, report_id: Uuid) -> Result<(), ReportError> {
    let report = self
      .report_repo
      .find_by_id(report_id)
      .await?
      .ok_or(ReportError::NotFound)?;

    self.transaction_repo.delete_by_report_id(report.id).await?;
    self.report_repo.delete(report.id).await?;

    Ok(())
  }

  /// Mark report as generated
  pub async fn mark_generated(
    &self,
    report_id: Uuid,
    drive_folder_id: String,
  ) -> Result<MonthlyReport, ReportError> {
    let mut report = self
      .report_repo
      .find_by_id(report_id)
      .await?
      .ok_or(ReportError::NotFound)?;

    report.status = ReportStatus::Generated;
    report.drive_folder_id = Some(drive_folder_id);
    report.updated_at = Utc::now();

    self.report_repo.update(report).await
  }

  // -- Received Invoices --

  /// Create a received invoice
  pub async fn create_received_invoice(
    &self,
    invoice: ReceivedInvoice,
  ) -> Result<ReceivedInvoice, ReportError> {
    self.received_invoice_repo.create(invoice).await
  }

  /// List received invoices for a company
  pub async fn list_received_invoices(
    &self,
    company_id: Uuid,
  ) -> Result<Vec<ReceivedInvoice>, ReportError> {
    self
      .received_invoice_repo
      .find_by_company_id(company_id)
      .await
  }

  /// Delete a received invoice
  pub async fn delete_received_invoice(&self, id: Uuid) -> Result<String, ReportError> {
    let invoice = self
      .received_invoice_repo
      .find_by_id(id)
      .await?
      .ok_or(ReportError::ReceivedInvoiceNotFound)?;

    let pdf_path = invoice.pdf_path.clone();
    self.received_invoice_repo.delete(id).await?;

    Ok(pdf_path)
  }

  /// Update receipt path on a transaction
  pub async fn update_receipt_path(
    &self,
    transaction_id: Uuid,
    receipt_path: Option<String>,
  ) -> Result<(), ReportError> {
    // Verify transaction exists
    let tx = self
      .transaction_repo
      .find_by_id(transaction_id)
      .await?
      .ok_or(ReportError::TransactionNotFound)?;

    self
      .transaction_repo
      .update_receipt_path(transaction_id, receipt_path)
      .await?;

    // Recalculate matched count since receipt affects is_matched()
    self.update_matched_count(tx.report_id).await?;

    Ok(())
  }

  /// Get a received invoice by ID
  pub async fn get_received_invoice(&self, id: Uuid) -> Result<ReceivedInvoice, ReportError> {
    self
      .received_invoice_repo
      .find_by_id(id)
      .await?
      .ok_or(ReportError::ReceivedInvoiceNotFound)
  }

  /// Auto-match received invoices to debit transactions by exact amount.
  /// Returns the number of auto-matched transactions.
  async fn auto_match_received_invoices(
    &self,
    _report_id: Uuid,
    company_id: Uuid,
    transactions: &[BankTransaction],
  ) -> Result<i32, ReportError> {
    let unmatched_invoices = self
      .received_invoice_repo
      .find_unmatched_by_company(company_id)
      .await?;

    if unmatched_invoices.is_empty() {
      return Ok(0);
    }

    let mut matched_count = 0i32;

    for tx in transactions {
      // Only auto-match debit (outgoing) transactions
      if tx.direction != TransactionDirection::Debit || tx.is_matched() {
        continue;
      }

      // Find received invoices with exact same amount
      let candidates: Vec<&ReceivedInvoice> = unmatched_invoices
        .iter()
        .filter(|inv| inv.amount == tx.amount && inv.currency == tx.currency)
        .collect();

      // Only auto-match if exactly one candidate (unambiguous)
      if candidates.len() == 1 {
        self
          .transaction_repo
          .update_match(tx.id, None, Some(candidates[0].id))
          .await?;
        matched_count += 1;
      }
    }

    Ok(matched_count)
  }

  /// Helper: recalculate matched count for a report
  async fn update_matched_count(&self, report_id: Uuid) -> Result<(), ReportError> {
    let transactions = self.transaction_repo.find_by_report_id(report_id).await?;
    let matched_count = transactions.iter().filter(|t| t.is_matched()).count() as i32;

    let mut report = self
      .report_repo
      .find_by_id(report_id)
      .await?
      .ok_or(ReportError::NotFound)?;

    report.matched_count = matched_count;
    report.updated_at = Utc::now();
    self.report_repo.update(report).await?;

    Ok(())
  }
}
