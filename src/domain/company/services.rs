use std::sync::Arc;
use uuid::Uuid;

use crate::domain::auth::{ports::UserRepository, value_objects::Email};

use super::{
  entities::{
    ActiveBankAccount, ActiveCompany, BankAccount, Company, CompanyMember, CompanyProfileUpdate,
    CompanyRole,
  },
  errors::CompanyError,
  ports::{
    ActiveBankAccountRepository, ActiveCompanyRepository, BankAccountRepository,
    CompanyMemberRepository, CompanyRepository,
  },
  value_objects::{BankAccountName, BankDetails, CompanyName, Iban},
};

/// Company service implementing core business logic
pub struct CompanyService {
  company_repo: Arc<dyn CompanyRepository>,
  member_repo: Arc<dyn CompanyMemberRepository>,
  active_repo: Arc<dyn ActiveCompanyRepository>,
  user_repo: Arc<dyn UserRepository>,
  bank_account_repo: Arc<dyn BankAccountRepository>,
  active_bank_account_repo: Arc<dyn ActiveBankAccountRepository>,
}

impl CompanyService {
  pub fn new(
    company_repo: Arc<dyn CompanyRepository>,
    member_repo: Arc<dyn CompanyMemberRepository>,
    active_repo: Arc<dyn ActiveCompanyRepository>,
    user_repo: Arc<dyn UserRepository>,
    bank_account_repo: Arc<dyn BankAccountRepository>,
    active_bank_account_repo: Arc<dyn ActiveBankAccountRepository>,
  ) -> Self {
    Self {
      company_repo,
      member_repo,
      active_repo,
      user_repo,
      bank_account_repo,
      active_bank_account_repo,
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
    profile: CompanyProfileUpdate,
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
    company.update_profile(profile);

    // Save
    self.company_repo.update(company).await
  }

  /// Update company storage configuration (requires owner/admin permission)
  pub async fn update_storage_config(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    storage_provider: Option<String>,
    storage_config: Option<String>,
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

    // Update storage configuration
    company.storage_provider = storage_provider;
    company.storage_config = storage_config;
    company.updated_at = chrono::Utc::now();

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

  // ===== Bank Account Management =====

  /// Create bank account (requires owner/admin permission)
  pub async fn create_bank_account(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    name: BankAccountName,
    iban: Iban,
    bank_details: Option<BankDetails>,
  ) -> Result<BankAccount, CompanyError> {
    // Verify requester can manage members (owner/admin)
    let member = self.verify_membership(company_id, requester_id).await?;
    if !member.can_manage_members() {
      return Err(CompanyError::InsufficientPermissions);
    }

    // Check for duplicate IBAN
    if self
      .bank_account_repo
      .find_by_iban(company_id, iban.as_str())
      .await?
      .is_some()
    {
      return Err(CompanyError::DuplicateIban);
    }

    // Create account
    let account = BankAccount::new(company_id, name, iban, bank_details);
    self.bank_account_repo.create(account).await
  }

  /// Update bank account (requires owner/admin permission)
  pub async fn update_bank_account(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    account_id: Uuid,
    name: BankAccountName,
    iban: Iban,
    bank_details: Option<BankDetails>,
  ) -> Result<BankAccount, CompanyError> {
    // Verify requester can manage members (owner/admin)
    let member = self.verify_membership(company_id, requester_id).await?;
    if !member.can_manage_members() {
      return Err(CompanyError::InsufficientPermissions);
    }

    // Get existing account
    let mut account = self
      .bank_account_repo
      .find_by_id(account_id)
      .await?
      .ok_or(CompanyError::BankAccountNotFound)?;

    // Verify account belongs to company
    if account.company_id != company_id {
      return Err(CompanyError::BankAccountNotFound);
    }

    // Check for duplicate IBAN (excluding current account)
    if let Some(existing) = self
      .bank_account_repo
      .find_by_iban(company_id, iban.as_str())
      .await?
    {
      if existing.id != account_id {
        return Err(CompanyError::DuplicateIban);
      }
    }

    // Update account
    account.update(name, iban, bank_details);
    self.bank_account_repo.update(account).await
  }

  /// Archive bank account (requires owner/admin, cannot archive active)
  pub async fn archive_bank_account(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    account_id: Uuid,
  ) -> Result<(), CompanyError> {
    // Verify requester can manage members (owner/admin)
    let member = self.verify_membership(company_id, requester_id).await?;
    if !member.can_manage_members() {
      return Err(CompanyError::InsufficientPermissions);
    }

    // Get account
    let mut account = self
      .bank_account_repo
      .find_by_id(account_id)
      .await?
      .ok_or(CompanyError::BankAccountNotFound)?;

    // Verify account belongs to company
    if account.company_id != company_id {
      return Err(CompanyError::BankAccountNotFound);
    }

    // Check if it's the active account
    if let Some(active_id) = self.active_bank_account_repo.get_active(company_id).await? {
      if active_id == account_id {
        return Err(CompanyError::CannotArchiveActiveBankAccount);
      }
    }

    // Archive
    account.archive();
    self.bank_account_repo.update(account).await?;
    Ok(())
  }

  /// Set active bank account (requires membership)
  pub async fn set_active_bank_account(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    account_id: Uuid,
  ) -> Result<(), CompanyError> {
    // Verify membership
    self.verify_membership(company_id, requester_id).await?;

    // Verify account exists and belongs to company
    let account = self
      .bank_account_repo
      .find_by_id(account_id)
      .await?
      .ok_or(CompanyError::BankAccountNotFound)?;

    if account.company_id != company_id {
      return Err(CompanyError::BankAccountNotFound);
    }

    // Cannot set archived account as active
    if account.is_archived() {
      return Err(CompanyError::BankAccountNotFound);
    }

    // Set as active
    let active = ActiveBankAccount::new(company_id, account_id);
    self.active_bank_account_repo.set_active(active).await
  }

  /// Get company bank accounts (all members can view)
  pub async fn get_company_bank_accounts(
    &self,
    company_id: Uuid,
    requester_id: Uuid,
    include_archived: bool,
  ) -> Result<Vec<BankAccount>, CompanyError> {
    // Verify membership
    self.verify_membership(company_id, requester_id).await?;

    // Get accounts
    self
      .bank_account_repo
      .find_by_company_id(company_id, include_archived)
      .await
  }
}
