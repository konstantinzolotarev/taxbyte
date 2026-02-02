use actix_web::{HttpMessage, HttpRequest, HttpResponse, web};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::adapters::http::{AuthErrorKind, errors::ApiError, templates::TemplateEngine};
use crate::application::company::GetUserCompaniesCommand;
use crate::application::invoice::{
  ArchiveCustomerCommand, ArchiveCustomerUseCase, CreateCustomerCommand, CreateCustomerUseCase,
  ListCustomersCommand, ListCustomersUseCase, UpdateCustomerCommand, UpdateCustomerUseCase,
};
use crate::domain::auth::entities::User;

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

// GET /customers - List all customers
pub async fn customers_page(
  req: HttpRequest,
  templates: web::Data<TemplateEngine>,
  list_customers_use_case: web::Data<Arc<ListCustomersUseCase>>,
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = get_active_company_id(user.id, &get_companies_use_case).await?;

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
  get_companies_use_case: web::Data<Arc<crate::application::company::GetUserCompaniesUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let company_id = get_active_company_id(user.id, &get_companies_use_case).await?;

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
        .insert_header(("HX-Redirect", "/customers"))
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

// POST /customers/{id}/edit - Update a customer
pub async fn update_customer_submit(
  req: HttpRequest,
  path: web::Path<Uuid>,
  form: web::Form<UpdateCustomerForm>,
  templates: web::Data<TemplateEngine>,
  update_customer_use_case: web::Data<Arc<UpdateCustomerUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let customer_id = path.into_inner();

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
        .insert_header(("HX-Redirect", "/customers"))
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

// DELETE /customers/{id}/archive - Archive a customer
pub async fn archive_customer(
  req: HttpRequest,
  path: web::Path<Uuid>,
  archive_customer_use_case: web::Data<Arc<ArchiveCustomerUseCase>>,
) -> Result<HttpResponse, ApiError> {
  let user = get_user(&req)?;
  let customer_id = path.into_inner();

  archive_customer_use_case
    .execute(ArchiveCustomerCommand {
      user_id: user.id,
      customer_id,
    })
    .await?;

  Ok(
    HttpResponse::Ok()
      .insert_header(("HX-Redirect", "/customers"))
      .finish(),
  )
}
