use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::{AuthErrorKind, errors::ApiError, templates::TemplateEngine};
use crate::application::company::{
  GetBankAccountsCommand, GetBankAccountsUseCase, GetUserCompaniesCommand,
};
use crate::application::invoice::{
  ArchiveInvoiceCommand, ArchiveInvoiceUseCase, ChangeInvoiceStatusCommand,
  ChangeInvoiceStatusUseCase, CreateInvoiceCommand, CreateInvoiceLineItemDto, CreateInvoiceUseCase,
  GetInvoiceDetailsCommand, GetInvoiceDetailsUseCase, ListCustomersCommand, ListCustomersUseCase,
  ListInvoicesCommand, ListInvoicesUseCase,
};
use crate::domain::auth::entities::User;
use crate::domain::company::ports::ActiveBankAccountRepository;

fn get_user(req: &HttpRequest) -> Result<User, ApiError> {
  req
    .extensions()
    .get::<User>()
    .cloned()
    .ok_or(ApiError::Auth(AuthErrorKind::InvalidSession))
}

async fn get_active_company_id(
  user_id: Uuid,
  get_companies_use_case: &Arc<crate::application::company::GetUserCompaniesUseCase>,
) -> Result<Uuid, ApiError> {
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id })
    .await?;

  companies_response
    .companies
    .iter()
    .find(|c| c.is_active)
    .map(|c| c.company_id)
    .ok_or_else(|| {
      ApiError::Validation("No active company selected. Please select a company.".to_string())
    })
}

// GET /invoices - List all invoices
pub async fn invoices_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_invoices_use_case: web::Data<Arc<ListInvoicesUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = get_active_company_id(user.id, &get_companies_use_case).await?;

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
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
  get_bank_accounts_use_case: web::Data<Arc<GetBankAccountsUseCase>>,
  active_bank_account_repo: web::Data<Arc<dyn ActiveBankAccountRepository>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = get_active_company_id(user.id, &get_companies_use_case).await?;

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
  context.insert("company_id", &company_id);
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
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = get_active_company_id(user.id, &get_companies_use_case).await?;

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
  path: web::Path<Uuid>,
  templates: web::Data<TemplateEngine>,
  get_invoice_details_use_case: web::Data<Arc<GetInvoiceDetailsUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let invoice_id = path.into_inner();

  let response = get_invoice_details_use_case
    .execute(GetInvoiceDetailsCommand {
      user_id: user.id,
      invoice_id,
    })
    .await?;

  let mut context = tera::Context::new();
  context.insert("invoice", &response);
  context.insert("user", &user);

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
  path: web::Path<Uuid>,
  form: web::Form<ChangeStatusForm>,
  change_status_use_case: web::Data<Arc<ChangeInvoiceStatusUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let invoice_id = path.into_inner();

  change_status_use_case
    .execute(ChangeInvoiceStatusCommand {
      user_id: user.id,
      invoice_id,
      new_status: form.status.clone(),
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", format!("/invoices/{}", invoice_id)))
      .finish(),
  )
}

// DELETE /invoices/{id}/archive - Archive an invoice
pub async fn archive_invoice(
  req: HttpRequest,
  path: web::Path<Uuid>,
  archive_invoice_use_case: web::Data<Arc<ArchiveInvoiceUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let invoice_id = path.into_inner();

  archive_invoice_use_case
    .execute(ArchiveInvoiceCommand {
      user_id: user.id,
      invoice_id,
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", "/invoices"))
      .finish(),
  )
}
