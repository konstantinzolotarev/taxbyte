use std::sync::Arc;
use uuid::Uuid;

use crate::domain::auth::{ports::UserRepository, value_objects::Email};

use super::{
  entities::{ActiveCompany, Company, CompanyMember, CompanyRole},
  errors::CompanyError,
  ports::{ActiveCompanyRepository, CompanyMemberRepository, CompanyRepository},
  value_objects::{CompanyAddress, CompanyName, PhoneNumber, RegistryCode, VatNumber},
};

/// Company service implementing core business logic
pub struct CompanyService {
  company_repo: Arc<dyn CompanyRepository>,
  member_repo: Arc<dyn CompanyMemberRepository>,
  active_repo: Arc<dyn ActiveCompanyRepository>,
  user_repo: Arc<dyn UserRepository>,
}

impl CompanyService {
  pub fn new(
    company_repo: Arc<dyn CompanyRepository>,
    member_repo: Arc<dyn CompanyMemberRepository>,
    active_repo: Arc<dyn ActiveCompanyRepository>,
    user_repo: Arc<dyn UserRepository>,
  ) -> Self {
    Self {
      company_repo,
      member_repo,
      active_repo,
      user_repo,
    }
  }

  /// Create new company with user as owner
  pub async fn create_company(
    &self,
    name: CompanyName,
    owner_id: Uuid,
  ) -> Result<Company, CompanyError> {
    // Create company
    let company = Company::new(name.into_inner());
    let created = self.company_repo.create(company).await?;

    // Add owner as first member
    let owner = CompanyMember::new(created.id, owner_id, CompanyRole::Owner);
    self.member_repo.add_member(owner).await?;

    Ok(created)
  }

  /// Set active company for user (validates membership)
  pub async fn set_active_company(
    &self,
    user_id: Uuid,
    company_id: Uuid,
  ) -> Result<(), CompanyError> {
    // Verify user is member
    self.verify_membership(company_id, user_id).await?;

    // Set as active
    let active = ActiveCompany::new(user_id, company_id);
    self.active_repo.set_active(active).await?;

    Ok(())
  }

  /// Add member to company (requires owner/admin permission)
  pub async fn add_member(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    new_member_email: Email,
    role: CompanyRole,
  ) -> Result<CompanyMember, CompanyError> {
    // Verify requester can manage members
    let requester = self.verify_membership(company_id, requester_id).await?;
    if !requester.can_manage_members() {
      return Err(CompanyError::InsufficientPermissions);
    }

    // Find user by email
    let user = self
      .user_repo
      .find_by_email(&new_member_email)
      .await?
      .ok_or(CompanyError::UserNotFound)?;

    // Check not already member
    if self
      .member_repo
      .find_member(company_id, user.id)
      .await?
      .is_some()
    {
      return Err(CompanyError::AlreadyMember);
    }

    // Add member
    let member = CompanyMember::new(company_id, user.id, role);
    self.member_repo.add_member(member).await
  }

  /// Remove member from company (requires owner/admin, can't remove last owner)
  pub async fn remove_member(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    member_id: Uuid,
  ) -> Result<(), CompanyError> {
    // Verify requester can manage members
    let requester = self.verify_membership(company_id, requester_id).await?;
    if !requester.can_manage_members() {
      return Err(CompanyError::InsufficientPermissions);
    }

    // Get member being removed
    let member = self
      .member_repo
      .find_member(company_id, member_id)
      .await?
      .ok_or(CompanyError::NotMember)?;

    // If removing an owner, ensure not last owner
    if member.is_owner() {
      let all_members = self.member_repo.find_by_company_id(company_id).await?;
      let owner_count = all_members.iter().filter(|m| m.is_owner()).count();

      if owner_count <= 1 {
        return Err(CompanyError::CannotRemoveLastOwner);
      }
    }

    // Remove member
    self.member_repo.remove_member(company_id, member_id).await
  }

  /// Get companies for user (where user is member)
  pub async fn get_user_companies(&self, user_id: Uuid) -> Result<Vec<Company>, CompanyError> {
    // Get all memberships
    let memberships = self.member_repo.find_by_user_id(user_id).await?;

    // Get companies
    let mut companies = Vec::new();
    for membership in memberships {
      if let Some(company) = self.company_repo.find_by_id(membership.company_id).await? {
        companies.push(company);
      }
    }

    Ok(companies)
  }

  /// Update company profile (requires owner/admin permission)
  pub async fn update_company_profile(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    email: Option<Email>,
    phone: Option<PhoneNumber>,
    address: Option<CompanyAddress>,
    registry_code: Option<RegistryCode>,
    vat_number: Option<VatNumber>,
  ) -> Result<Company, CompanyError> {
    // Verify requester is member
    let member = self.verify_membership(company_id, requester_id).await?;

    // Check permissions (owner or admin)
    if !member.can_manage_members() {
      return Err(CompanyError::InsufficientPermissions);
    }

    // Get company
    let mut company = self
      .company_repo
      .find_by_id(company_id)
      .await?
      .ok_or(CompanyError::NotFound)?;

    // Update profile
    company.update_profile(email, phone, address, registry_code, vat_number);

    // Save
    self.company_repo.update(company).await
  }

  /// Helper: Verify user is member of company
  async fn verify_membership(
    &self,
    company_id: Uuid,
    user_id: Uuid,
  ) -> Result<CompanyMember, CompanyError> {
    self
      .member_repo
      .find_member(company_id, user_id)
      .await?
      .ok_or(CompanyError::NotMember)
  }
}
