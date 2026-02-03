use actix_web::{HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::handlers::{get_company_context, get_user};
use crate::adapters::http::{errors::ApiError, templates::TemplateEngine};
use crate::application::company::GetUserCompaniesCommand;
use crate::application::invoice::{
  ArchiveCustomerCommand, ArchiveCustomerUseCase, CreateCustomerCommand, CreateCustomerUseCase,
  ListCustomersCommand, ListCustomersUseCase, UpdateCustomerCommand, UpdateCustomerUseCase,
};

// GET /customers - List all customers
pub async fn customers_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_customers_use_case: web::Data<Arc<ListCustomersUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;

  // Extract company context from URL (validated by middleware)
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  // Fetch user's companies for the navbar selector
  let companies_response = get_companies_use_case
    .execute(GetUserCompaniesCommand { user_id: user.id })
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

  let response = list_customers_use_case
    .execute(ListCustomersCommand {
      user_id: user.id,
      company_id,
      include_archived: false,
    })
    .await?;

  let mut context = tera::Context::new();
  context.insert("customers", &response.customers);
  context.insert("user", &user);
  context.insert("companies", &companies_response.companies);
  context.insert("active_company", &active_company);
  context.insert("company_id", &company_id.to_string());
  context.insert("current_page", "customers");

  let html = templates
    .render("pages/customers.html.tera", &context)
    .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

  Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateCustomerForm {
  name: String,
  street: Option<String>,
  city: Option<String>,
  state: Option<String>,
  postal_code: Option<String>,
  country: Option<String>,
}

// POST /customers/create - Create a new customer
pub async fn create_customer_submit(
  req: HttpRequest,
  form: web::Form<CreateCustomerForm>,
  templates: web::Data<TemplateEngine>,
  create_customer_use_case: web::Data<Arc<CreateCustomerUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;
  let company_id = company_context.company_id;

  match create_customer_use_case
    .execute(CreateCustomerCommand {
      user_id: user.id,
      company_id,
      name: form.name.clone(),
      street: form.street.clone(),
      city: form.city.clone(),
      state: form.state.clone(),
      postal_code: form.postal_code.clone(),
      country: form.country.clone(),
    })
    .await
  {
    Ok(_) => Ok(
      HttpResponse::Ok()
        .insert_header(("HX-Redirect", format!("/c/{}/customers", company_id)))
        .finish(),
    ),
    Err(e) => {
      let mut context = tera::Context::new();
      context.insert("error", &e.to_string());
      context.insert("form", &*form);

      let html = templates
        .render("partials/create_customer_form.html.tera", &context)
        .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateCustomerForm {
  name: String,
  street: Option<String>,
  city: Option<String>,
  state: Option<String>,
  postal_code: Option<String>,
  country: Option<String>,
}

// POST /c/{company_id}/customers/{id}/edit - Update a customer
pub async fn update_customer_submit(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  form: web::Form<UpdateCustomerForm>,
  templates: web::Data<TemplateEngine>,
  update_customer_use_case: web::Data<Arc<UpdateCustomerUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;

  let (company_id, customer_id) = path.into_inner();

  // Verify the company_id from URL matches the context
  if company_id != company_context.company_id {
    return Err(ApiError::Auth(
      crate::adapters::http::errors::AuthErrorKind::Forbidden,
    ));
  }

  match update_customer_use_case
    .execute(UpdateCustomerCommand {
      user_id: user.id,
      customer_id,
      name: form.name.clone(),
      street: form.street.clone(),
      city: form.city.clone(),
      state: form.state.clone(),
      postal_code: form.postal_code.clone(),
      country: form.country.clone(),
    })
    .await
  {
    Ok(_) => Ok(
      HttpResponse::Ok()
        .insert_header(("HX-Redirect", format!("/c/{}/customers", company_id)))
        .finish(),
    ),
    Err(e) => {
      let mut context = tera::Context::new();
      context.insert("error", &e.to_string());
      context.insert("form", &*form);
      context.insert("customer_id", &customer_id);

      let html = templates
        .render("partials/edit_customer_form.html.tera", &context)
        .map_err(|e| ApiError::Internal(format!("Template error: {}", e)))?;

      Ok(
        HttpResponse::BadRequest()
          .content_type("text/html")
          .body(html),
      )
    }
  }
}

// DELETE /c/{company_id}/customers/{id}/archive - Archive a customer
pub async fn archive_customer(
  req: HttpRequest,
  path: web::Path<(Uuid, Uuid)>,
  archive_customer_use_case: web::Data<Arc<ArchiveCustomerUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_context = get_company_context(&req)?;

  let (company_id, customer_id) = path.into_inner();

  // Verify the company_id from URL matches the context
  if company_id != company_context.company_id {
    return Err(ApiError::Auth(
      crate::adapters::http::errors::AuthErrorKind::Forbidden,
    ));
  }

  archive_customer_use_case
    .execute(ArchiveCustomerCommand {
      user_id: user.id,
      customer_id,
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", format!("/c/{}/customers", company_id)))
      .finish(),
  )
}
