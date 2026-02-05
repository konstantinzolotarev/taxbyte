use actix_web::{HttpRequest, HttpResponse, web};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::{
  errors::ApiError,
  handlers::{get_company_context, get_user},
  templates::TemplateEngine,
};
use crate::application::company::{GetBankAccountsCommand, GetBankAccountsUseCase};
use crate::application::invoice::{
  ArchiveInvoiceCommand, ArchiveInvoiceUseCase, ArchiveTemplateCommand, ArchiveTemplateUseCase,
  ChangeInvoiceStatusCommand, ChangeInvoiceStatusUseCase, CreateInvoiceCommand,
  CreateInvoiceFromTemplateCommand, CreateInvoiceFromTemplateUseCase, CreateInvoiceLineItemDto,
  CreateInvoiceUseCase, CreateTemplateFromInvoiceCommand, CreateTemplateFromInvoiceUseCase,
  DeleteInvoiceCommand, DeleteInvoiceUseCase, GetInvoiceDetailsCommand, GetInvoiceDetailsUseCase,
  ListCustomersCommand, ListCustomersUseCase, ListInvoicesCommand, ListInvoicesUseCase,
  ListTemplatesCommand, ListTemplatesUseCase,
};
use crate::domain::company::ports::ActiveBankAccountRepository;

// GET /invoices - List all invoices
pub async fn invoices_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_invoices_use_case: web::Data<Arc<ListInvoicesUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for the navbar selector
  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Find current company from the list for the selector
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

  let response = list_invoices_use_case
    .execute(ListInvoicesCommand {
      user_id: user.id,
      company_id,
      status_filter: None,
      customer_filter: None,
    })
    .await?;

  let mut context = tera::Context::new();
  context.insert("invoices", &response.invoices);
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "invoices");

  let html = templates
    .render("pages/invoices.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

// GET /invoices/create - Show invoice creation form
pub async fn invoice_create_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_customers_use_case: web::Data<Arc<ListCustomersUseCase>>,
  get_bank_accounts_use_case: web::Data<Arc<GetBankAccountsUseCase>>,
  active_bank_account_repo: web::Data<Arc<dyn ActiveBankAccountRepository>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for the navbar selector
  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Find current company from the list for the selector
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

  let customers_response = list_customers_use_case
    .execute(ListCustomersCommand {
      user_id: user.id,
      company_id,
      include_archived: false,
    })
    .await?;

  // Fetch bank accounts for dropdown
  let bank_accounts_response = get_bank_accounts_use_case
    .execute(GetBankAccountsCommand {
      company_id,
      requester_id: user.id,
      include_archived: false,
    })
    .await?;

  // Get active bank account ID
  let active_bank_account_id = active_bank_account_repo
    .get_active(company_id)
    .await
    .ok()
    .flatten();

  let mut context = tera::Context::new();
  context.insert("customers", &customers_response.customers);
  context.insert("bank_accounts", &bank_accounts_response.accounts);
  context.insert("active_bank_account_id", &active_bank_account_id);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "invoices");
  context.insert("user", &user);

  let html = templates
    .render("pages/invoice_create.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceFormLineItem {
  description: String,
  quantity: Decimal,
  unit_price: Decimal,
  vat_rate: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceForm {
  customer_id: Uuid,
  invoice_number: String,
  invoice_date: NaiveDate,
  payment_terms: String,
  currency: String,
  line_items: Vec<CreateInvoiceFormLineItem>,
  bank_account_id: Option<Uuid>,
}

// POST /invoices/create - Create a new invoice
pub async fn create_invoice_submit(
  req: HttpRequest,
  form: web::Json<CreateInvoiceForm>,
  create_invoice_use_case: web::Data<Arc<CreateInvoiceUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  let line_items = form
    .line_items
    .iter()
    .map(|item| CreateInvoiceLineItemDto {
      description: item.description.clone(),
      quantity: item.quantity,
      unit_price: item.unit_price,
      vat_rate: item.vat_rate,
    })
    .collect();

  let response = create_invoice_use_case
    .execute(CreateInvoiceCommand {
      user_id: user.id,
      company_id,
      customer_id: form.customer_id,
      bank_account_id: form.bank_account_id,
      invoice_number: form.invoice_number.clone(),
      invoice_date: form.invoice_date,
      payment_terms: form.payment_terms.clone(),
      currency: form.currency.clone(),
      line_items,
    })
    .await?;

  Ok(HttpResponse::Ok().json(response))
}

// GET /invoices/{id} - Show invoice details
pub async fn invoice_details_page(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  templates: web::Data<TemplateEngine>,
  get_invoice_details_use_case: web::Data<Arc<GetInvoiceDetailsUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (_company_id_from_path, invoice_id) = path.into_inner();
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for the navbar selector
  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Find current company from the list for the selector
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

  let response = get_invoice_details_use_case
    .execute(GetInvoiceDetailsCommand {
      user_id: user.id,
      invoice_id,
    })
    .await?;

  let mut context = tera::Context::new();
  context.insert("invoice", &response);
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "invoices");

  let html = templates
    .render("pages/invoice_details.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize)]
pub struct ChangeStatusForm {
  status: String,
}

// POST /invoices/{id}/status - Change invoice status
pub async fn change_invoice_status(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  form: web::Form<ChangeStatusForm>,
  change_status_use_case: web::Data<Arc<ChangeInvoiceStatusUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (_company_id_from_path, invoice_id) = path.into_inner();
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  change_status_use_case
    .execute(ChangeInvoiceStatusCommand {
      user_id: user.id,
      invoice_id,
      new_status: form.status.clone(),
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/invoices/{}", company_id, invoice_id),
      ))
      .finish(),
  )
}

// DELETE /invoices/{id}/archive - Archive an invoice
pub async fn archive_invoice(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  archive_invoice_use_case: web::Data<Arc<ArchiveInvoiceUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (_company_id_from_path, invoice_id) = path.into_inner();
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  archive_invoice_use_case
    .execute(ArchiveInvoiceCommand {
      user_id: user.id,
      invoice_id,
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", format!("/c/{}/invoices", company_id)))
      .finish(),
  )
}

// DELETE /c/{company_id}/invoices/{id} - Delete an invoice
pub async fn delete_invoice(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  delete_invoice_use_case: web::Data<Arc<DeleteInvoiceUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (_company_id_from_path, invoice_id) = path.into_inner();
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  delete_invoice_use_case
    .execute(DeleteInvoiceCommand {
      user_id: user.id,
      invoice_id,
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", format!("/c/{}/invoices", company_id)))
      .finish(),
  )
}

// GET /c/{company_id}/invoices/templates - List invoice templates
pub async fn templates_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_templates_use_case: web::Data<Arc<ListTemplatesUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for the navbar selector
  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Find current company from the list for the selector
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

  let response = list_templates_use_case
    .execute(ListTemplatesCommand {
      user_id: user.id,
      company_id,
      include_archived: false,
    })
    .await?;

  let mut context = tera::Context::new();
  context.insert("templates", &response.templates);
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "invoices");

  let html = templates
    .render("pages/invoice_templates.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize)]
pub struct SaveAsTemplateForm {
  name: String,
  description: Option<String>,
}

// POST /c/{company_id}/invoices/{id}/save-as-template - Save invoice as template
pub async fn save_as_template(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  form: web::Form<SaveAsTemplateForm>,
  create_template_use_case: web::Data<Arc<CreateTemplateFromInvoiceUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (_company_id_from_path, invoice_id) = path.into_inner();
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  create_template_use_case
    .execute(CreateTemplateFromInvoiceCommand {
      user_id: user.id,
      invoice_id,
      template_name: form.name.clone(),
      description: form.description.clone(),
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/invoices/templates", company_id),
      ))
      .finish(),
  )
}

// GET /c/{company_id}/invoices/create-from-template/{id} - Show create from template form
pub async fn create_from_template_page(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  templates: web::Data<TemplateEngine>,
  list_customers_use_case: web::Data<Arc<ListCustomersUseCase>>,
  get_bank_accounts_use_case: web::Data<Arc<GetBankAccountsUseCase>>,
  active_bank_account_repo: web::Data<Arc<dyn ActiveBankAccountRepository>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (_company_id_from_path, template_id) = path.into_inner();
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for the navbar selector
  let companies_response = get_companies_use_case
    .execute(crate::application::company::GetUserCompaniesCommand { user_id: user.id })
    .await?;

  // Find current company from the list for the selector
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

  let customers_response = list_customers_use_case
    .execute(ListCustomersCommand {
      user_id: user.id,
      company_id,
      include_archived: false,
    })
    .await?;

  // Fetch bank accounts for dropdown
  let bank_accounts_response = get_bank_accounts_use_case
    .execute(GetBankAccountsCommand {
      company_id,
      requester_id: user.id,
      include_archived: false,
    })
    .await?;

  // Get active bank account ID
  let active_bank_account_id = active_bank_account_repo
    .get_active(company_id)
    .await
    .ok()
    .flatten();

  let mut context = tera::Context::new();
  context.insert("template_id", &template_id.to_string());
  context.insert("customers", &customers_response.customers);
  context.insert("bank_accounts", &bank_accounts_response.accounts);
  context.insert("active_bank_account_id", &active_bank_account_id);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "invoices");
  context.insert("user", &user);
  context.insert(
    "today",
    &chrono::Local::now().format("%Y-%m-%d").to_string(),
  );

  let html = templates
    .render("pages/invoice_create_from_template.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize)]
pub struct CreateFromTemplateForm {
  invoice_number: String,
  invoice_date: NaiveDate,
}

// POST /c/{company_id}/invoices/create-from-template/{id} - Create invoice from template
pub async fn create_invoice_from_template(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  form: web::Form<CreateFromTemplateForm>,
  create_invoice_use_case: web::Data<Arc<CreateInvoiceFromTemplateUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;
  let (_company_id_from_path, template_id) = path.into_inner();

  let response = create_invoice_use_case
    .execute(CreateInvoiceFromTemplateCommand {
      user_id: user.id,
      template_id,
      invoice_number: form.invoice_number.clone(),
      invoice_date: form.invoice_date,
    })
    .await?;

  // Redirect to the newly created invoice
  Ok(
    HttpResponse::Found()
      .insert_header((
        "Location",
        format!("/c/{}/invoices/{}", company_id, response.invoice_id),
      ))
      .finish(),
  )
}

// DELETE /c/{company_id}/invoices/templates/{id} - Archive a template
pub async fn archive_template(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  archive_template_use_case: web::Data<Arc<ArchiveTemplateUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let (_company_id_from_path, template_id) = path.into_inner();
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  archive_template_use_case
    .execute(ArchiveTemplateCommand {
      user_id: user.id,
      template_id,
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header((
        "HX-Redirect",
        format!("/c/{}/invoices/templates", company_id),
      ))
      .finish(),
  )
}

// GET /invoices/{id}/html - Render invoice HTML for PDF generation
//
// SECURITY: This endpoint is restricted to localhost only (IP whitelist)
// Used by wkhtmltopdf for PDF generation. Only accepts requests from:
// - 127.0.0.1 (IPv4 localhost)
// - ::1 (IPv6 localhost)
//
pub async fn invoice_html_view(
  req: HttpRequest,
  path: web::Path<Uuid>,
  templates: web::Data<TemplateEngine>,
  get_invoice_details: web::Data<Arc<GetInvoiceDetailsUseCase>>,
) -> Result<HttpResponse, ApiError> {
  // IP Whitelist: Only allow localhost
  let peer_addr = req
    .peer_addr()
    .ok_or_else(|| ApiError::Internal("Cannot determine peer address".to_string()))?;

  let is_localhost = peer_addr.ip().is_loopback();

  if !is_localhost {
    tracing::warn!(
      "Rejected invoice HTML access from non-localhost IP: {}",
      peer_addr.ip()
    );
    return Err(ApiError::Auth(
      crate::adapters::http::errors::AuthErrorKind::Forbidden,
    ));
  }

  let invoice_id = path.into_inner();

  // Using nil UUID to bypass authentication checks (safe because IP is whitelisted)
  let system_user_id = Uuid::nil();

  // Get invoice details
  let invoice_data = get_invoice_details
    .execute(GetInvoiceDetailsCommand {
      user_id: system_user_id,
      invoice_id,
    })
    .await?;

  // Render the invoice PDF template
  let mut context = tera::Context::new();
  context.insert("invoice", &invoice_data);

  let html = templates
    .render("partials/invoice_pdf.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}
