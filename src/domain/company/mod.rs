pub mod entities;
pub mod errors;
pub mod ports;
pub mod services;
pub mod value_objects;

pub use entities::{ActiveCompany, Company, CompanyMember, CompanyProfileUpdate, CompanyRole};
pub use errors::{CompanyError, ValidationError};
pub use ports::{ActiveCompanyRepository, CompanyMemberRepository, CompanyRepository};
pub use services::CompanyService;
pub use value_objects::{CompanyAddress, CompanyName, PhoneNumber, RegistryCode, VatNumber};
