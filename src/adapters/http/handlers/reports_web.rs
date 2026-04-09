use actix_multipart::Multipart;
use actix_web::{HttpRequest, HttpResponse, web};
use chrono::NaiveDate;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::{
  errors::ApiError,
  handlers::{get_company_context, get_user},
  templates::TemplateEngine,
};
use crate::application::report::{
  CreateEmptyReportCommand, CreateEmptyReportUseCase, DeleteReceivedInvoiceCommand,
  DeleteReceivedInvoiceUseCase, DeleteReportCommand, DeleteReportUseCase, GetReportDetailsCommand,
  GetReportDetailsUseCase, ImportBankStatementCommand, ImportBankStatementUseCase,
  ListMonthlyReportsCommand, ListMonthlyReportsUseCase, ListReceivedInvoicesCommand,
  ListReceivedInvoicesUseCase, MatchTransactionCommand, MatchTransactionUseCase,
  UnmatchTransactionCommand, UnmatchTransactionUseCase, UploadReceiptCommand, UploadReceiptUseCase,
  UploadReceivedInvoiceCommand, UploadReceivedInvoiceUseCase,
};
use crate::domain::report::ports::InvoiceDataExtractor;

// GET /reports - List monthly reports
pub async fn reports_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_reports_use_case: web::Data<Arc<ListMonthlyReportsUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  let response = list_reports_use_case
    .execute(ListMonthlyReportsCommand { company_id })
    .await
    .map_err(ApiError::from)?;

  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.company_id == company_id)
    .map(|c| {
      serde_json::json!({
        "company_id": c.company_id,
        "name": c.name,
        "role": c.role,
      })
    });

  let mut context = tera::Context::new();
  context.insert("reports", &response.reports);
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "reports");

  let html = templates
    .render("pages/reports.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {:?}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// GET /reports/create - CSV upload form
pub async fn create_report_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.company_id == company_id)
    .map(|c| {
      serde_json::json!({
        "company_id": c.company_id,
        "name": c.name,
        "role": c.role,
      })
    });

  let mut context = tera::Context::new();
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "reports");

  let html = templates
    .render("pages/report_create.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {:?}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize)]
pub struct CreateEmptyReportForm {
  pub month: u32,
  pub year: i32,
}

// POST /reports/create-empty - Create empty report shell
pub async fn create_empty_report(
  req: HttpRequest,
  form: web::Form<CreateEmptyReportForm>,
  create_use_case: web::Data<Arc<CreateEmptyReportUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  let result = create_use_case
    .execute(CreateEmptyReportCommand {
      company_id,
      month: form.month,
      year: form.year,
    })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/{}", company_id, result.report_id),
      ))
      .finish(),
  )
}

// POST /reports/import - Import bank statement CSV
pub async fn import_bank_statement(
  req: HttpRequest,
  mut payload: Multipart,
  import_use_case: web::Data<Arc<ImportBankStatementUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  let mut csv_content: Option<Vec<u8>> = None;
  let mut month: Option<u32> = None;
  let mut year: Option<i32> = None;

  while let Some(item) = payload.next().await {
    let mut field = item.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
    let field_name = field.name().map(|s| s.to_string()).unwrap_or_default();

    let mut bytes = Vec::new();
    while let Some(chunk) = field.next().await {
      let data = chunk.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
      bytes.extend_from_slice(&data);
    }

    match field_name.as_str() {
      "csv_file" => {
        csv_content = Some(bytes);
      }
      "month" => {
        let s = String::from_utf8_lossy(&bytes);
        month = Some(
          s.trim()
            .parse::<u32>()
            .map_err(|_| ApiError::Validation("Invalid month".to_string()))?,
        );
      }
      "year" => {
        let s = String::from_utf8_lossy(&bytes);
        year = Some(
          s.trim()
            .parse::<i32>()
            .map_err(|_| ApiError::Validation("Invalid year".to_string()))?,
        );
      }
      _ => {}
    }
  }

  let csv_content =
    csv_content.ok_or_else(|| ApiError::Validation("CSV file is required".to_string()))?;
  let month = month.ok_or_else(|| ApiError::Validation("Month is required".to_string()))?;
  let year = year.ok_or_else(|| ApiError::Validation("Year is required".to_string()))?;

  let result = import_use_case
    .execute(ImportBankStatementCommand {
      company_id,
      month,
      year,
      csv_content,
    })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/{}", company_id, result.report_id),
      ))
      .finish(),
  )
}

// GET /reports/{id} - Report details page
pub async fn report_details_page(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  templates: web::Data<TemplateEngine>,
  get_details_use_case: web::Data<Arc<GetReportDetailsUseCase>>,
  list_received_use_case: web::Data<Arc<ListReceivedInvoicesUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
  list_invoices_use_case: web::Data<Arc<crate::application::invoice::ListInvoicesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_, report_id) = path.into_inner();

  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  let report = get_details_use_case
    .execute(GetReportDetailsCommand { report_id })
    .await
    .map_err(ApiError::from)?;

  // Get received invoices for matching
  let received = list_received_use_case
    .execute(ListReceivedInvoicesCommand { company_id })
    .await
    .map_err(ApiError::from)?;

  // Get issued invoices for matching
  let invoices = list_invoices_use_case
    .execute(crate::application::invoice::ListInvoicesCommand {
      user_id: user.id,
      company_id,
      status_filter: None,
      customer_filter: None,
    })
    .await?;

  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.company_id == company_id)
    .map(|c| {
      serde_json::json!({
        "company_id": c.company_id,
        "name": c.name,
        "role": c.role,
      })
    });

  let mut context = tera::Context::new();
  context.insert("report", &report);
  context.insert("transactions", &report.transactions);
  context.insert("received_invoices", &received.invoices);
  context.insert("invoices", &invoices.invoices);
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "reports");

  let html = templates
    .render("pages/report_details.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {:?}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize)]
pub struct MatchForm {
  pub invoice_id: Option<String>,
  pub received_invoice_id: Option<String>,
}

// POST /reports/{id}/match/{tx_id} - Match transaction
pub async fn match_transaction(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid, Uuid)>,
  form: web::Form<MatchForm>,
  match_use_case: web::Data<Arc<MatchTransactionUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_, _report_id, tx_id) = path.into_inner();

  let invoice_id = form
    .invoice_id
    .as_ref()
    .filter(|s| !s.is_empty())
    .map(|s| Uuid::parse_str(s))
    .transpose()
    .map_err(|_| ApiError::Validation("Invalid invoice ID".to_string()))?;

  let received_invoice_id = form
    .received_invoice_id
    .as_ref()
    .filter(|s| !s.is_empty())
    .map(|s| Uuid::parse_str(s))
    .transpose()
    .map_err(|_| ApiError::Validation("Invalid received invoice ID".to_string()))?;

  match_use_case
    .execute(MatchTransactionCommand {
      transaction_id: tx_id,
      invoice_id,
      received_invoice_id,
    })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/{}", company_id, _report_id),
      ))
      .finish(),
  )
}

// DELETE /reports/{id}/match/{tx_id} - Unmatch transaction
pub async fn unmatch_transaction(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid, Uuid)>,
  unmatch_use_case: web::Data<Arc<UnmatchTransactionUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_, report_id, tx_id) = path.into_inner();

  unmatch_use_case
    .execute(UnmatchTransactionCommand {
      transaction_id: tx_id,
    })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/{}", company_id, report_id),
      ))
      .finish(),
  )
}

// POST /reports/{id}/generate - Generate Drive report
pub async fn generate_report(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  generate_use_case: web::Data<Arc<crate::application::report::GenerateReportUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_, report_id) = path.into_inner();

  generate_use_case
    .execute(crate::application::report::GenerateReportCommand {
      report_id,
      company_id,
    })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/{}", company_id, report_id),
      ))
      .finish(),
  )
}

// DELETE /reports/{id} - Delete report
pub async fn delete_report(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  delete_use_case: web::Data<Arc<DeleteReportUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_, report_id) = path.into_inner();

  delete_use_case
    .execute(DeleteReportCommand { report_id })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", format!("/c/{}/reports", company_id)))
      .finish(),
  )
}

// GET /reports/received-invoices - List received invoices
pub async fn received_invoices_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_use_case: web::Data<Arc<ListReceivedInvoicesUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  let response = list_use_case
    .execute(ListReceivedInvoicesCommand { company_id })
    .await
    .map_err(ApiError::from)?;

  let active_company = companies_response
    .companies
    .iter()
    .find(|c| c.company_id == company_id)
    .map(|c| {
      serde_json::json!({
        "company_id": c.company_id,
        "name": c.name,
        "role": c.role,
      })
    });

  let mut context = tera::Context::new();
  context.insert("received_invoices", &response.invoices);
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "reports");
  // Default empty values for the included form fields partial
  context.insert("vendor_name", "");
  context.insert("amount", "");
  context.insert("currency", "");
  context.insert("invoice_number", "");
  context.insert("invoice_date", "");

  let html = templates
    .render("pages/received_invoices.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {:?}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// POST /reports/received-invoices - Upload received invoice
pub async fn upload_received_invoice(
  req: HttpRequest,
  mut payload: Multipart,
  upload_use_case: web::Data<Arc<UploadReceivedInvoiceUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  let mut pdf_bytes: Option<Vec<u8>> = None;
  let mut vendor_name = String::new();
  let mut amount_str = String::new();
  let mut currency = "EUR".to_string();
  let mut invoice_date_str = String::new();
  let mut invoice_number = String::new();
  let mut notes = String::new();

  while let Some(item) = payload.next().await {
    let mut field = item.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
    let field_name = field.name().map(|s| s.to_string()).unwrap_or_default();

    let mut bytes = Vec::new();
    while let Some(chunk) = field.next().await {
      let data = chunk.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
      bytes.extend_from_slice(&data);
    }

    match field_name.as_str() {
      "pdf_file" => pdf_bytes = Some(bytes),
      "vendor_name" => vendor_name = String::from_utf8_lossy(&bytes).trim().to_string(),
      "amount" => amount_str = String::from_utf8_lossy(&bytes).trim().to_string(),
      "currency" => currency = String::from_utf8_lossy(&bytes).trim().to_string(),
      "invoice_date" => invoice_date_str = String::from_utf8_lossy(&bytes).trim().to_string(),
      "invoice_number" => invoice_number = String::from_utf8_lossy(&bytes).trim().to_string(),
      "notes" => notes = String::from_utf8_lossy(&bytes).trim().to_string(),
      _ => {}
    }
  }

  let pdf_bytes =
    pdf_bytes.ok_or_else(|| ApiError::Validation("PDF file is required".to_string()))?;

  if vendor_name.is_empty() {
    return Err(ApiError::Validation("Vendor name is required".to_string()));
  }

  let amount = Decimal::from_str(&amount_str)
    .map_err(|_| ApiError::Validation("Invalid amount".to_string()))?;

  let invoice_date = if invoice_date_str.is_empty() {
    None
  } else {
    Some(
      NaiveDate::parse_from_str(&invoice_date_str, "%Y-%m-%d")
        .map_err(|_| ApiError::Validation("Invalid invoice date".to_string()))?,
    )
  };

  let inv_number = if invoice_number.is_empty() {
    None
  } else {
    Some(invoice_number)
  };

  let inv_notes = if notes.is_empty() { None } else { Some(notes) };

  // Save PDF to disk
  let pdf_dir = format!("data/received_invoices/{}", company_id);
  tokio::fs::create_dir_all(&pdf_dir)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to create directory: {}", e)))?;

  let file_id = Uuid::new_v4();
  let pdf_path = format!("{}/{}.pdf", pdf_dir, file_id);
  tokio::fs::write(&pdf_path, &pdf_bytes)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to save PDF: {}", e)))?;

  upload_use_case
    .execute(UploadReceivedInvoiceCommand {
      company_id,
      vendor_name,
      amount,
      currency,
      invoice_date,
      invoice_number: inv_number,
      pdf_path,
      notes: inv_notes,
    })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/received-invoices", company_id),
      ))
      .finish(),
  )
}

// POST /reports/{id}/receipt/{tx_id} - Upload receipt to transaction
pub async fn upload_receipt(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid, Uuid)>,
  mut payload: Multipart,
  upload_receipt_use_case: web::Data<Arc<UploadReceiptUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_, report_id, tx_id) = path.into_inner();

  let mut file_bytes: Option<Vec<u8>> = None;
  let mut file_ext = String::from("bin");

  while let Some(item) = payload.next().await {
    let mut field = item.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
    let field_name = field.name().map(|s| s.to_string()).unwrap_or_default();

    if field_name == "file" {
      // Extract extension from filename
      if let Some(cd) = field.content_disposition() {
        if let Some(filename) = cd.get_filename() {
          if let Some(ext) = filename.rsplit('.').next() {
            file_ext = ext.to_lowercase();
          }
        }
      }

      let mut bytes = Vec::new();
      while let Some(chunk) = field.next().await {
        let data = chunk.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
        bytes.extend_from_slice(&data);
      }
      file_bytes = Some(bytes);
    }
  }

  let file_bytes =
    file_bytes.ok_or_else(|| ApiError::Validation("File is required".to_string()))?;

  if file_bytes.is_empty() {
    return Err(ApiError::Validation("File is empty".to_string()));
  }

  // Save file to disk
  let receipt_dir = format!("data/receipts/{}", company_id);
  tokio::fs::create_dir_all(&receipt_dir)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to create directory: {}", e)))?;

  let file_id = Uuid::new_v4();
  let receipt_path = format!("{}/{}.{}", receipt_dir, file_id, file_ext);
  tokio::fs::write(&receipt_path, &file_bytes)
    .await
    .map_err(|e| ApiError::Internal(format!("Failed to save receipt: {}", e)))?;

  upload_receipt_use_case
    .execute(UploadReceiptCommand {
      transaction_id: tx_id,
      receipt_path,
    })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/{}", company_id, report_id),
      ))
      .finish(),
  )
}

// POST /reports/received-invoices/extract - Extract data from PDF
pub async fn extract_invoice_data(
  req: HttpRequest,
  mut payload: Multipart,
  templates: web::Data<TemplateEngine>,
  extractor: web::Data<Arc<dyn InvoiceDataExtractor>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;

  let mut pdf_bytes: Option<Vec<u8>> = None;

  while let Some(item) = payload.next().await {
    let mut field = item.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
    let field_name = field.name().map(|s| s.to_string()).unwrap_or_default();

    if field_name == "pdf_file" {
      let mut bytes = Vec::new();
      while let Some(chunk) = field.next().await {
        let data = chunk.map_err(|e| ApiError::Validation(format!("Upload error: {}", e)))?;
        bytes.extend_from_slice(&data);
      }
      if !bytes.is_empty() {
        pdf_bytes = Some(bytes);
      }
    }
  }

  let pdf_bytes =
    pdf_bytes.ok_or_else(|| ApiError::Validation("PDF file is required".to_string()))?;

  // Run extraction on blocking thread (CPU-bound)
  let extractor = extractor.into_inner();
  let extracted = tokio::task::spawn_blocking(move || extractor.extract(&pdf_bytes))
    .await
    .map_err(|e| ApiError::Internal(format!("Extraction task failed: {}", e)))?
    .unwrap_or_default();

  let mut context = tera::Context::new();
  context.insert("vendor_name", &extracted.vendor_name.unwrap_or_default());
  context.insert("amount", &extracted.amount.unwrap_or_default());
  context.insert("currency", &extracted.currency.unwrap_or_default());
  context.insert(
    "invoice_number",
    &extracted.invoice_number.unwrap_or_default(),
  );
  context.insert("invoice_date", &extracted.invoice_date.unwrap_or_default());

  let html = templates
    .render("partials/received_invoice_form_fields.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {:?}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// DELETE /reports/received-invoices/{id} - Delete received invoice
pub async fn delete_received_invoice(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  delete_use_case: web::Data<Arc<DeleteReceivedInvoiceUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let _user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_, id) = path.into_inner();

  delete_use_case
    .execute(DeleteReceivedInvoiceCommand { id })
    .await
    .map_err(ApiError::from)?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/reports/received-invoices", company_id),
      ))
      .finish(),
  )
}
