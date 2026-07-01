//! Persistence for compliance cases, behind the [`CaseRepository`] trait so
//! callers (server functions, future crawler/extraction jobs) never depend on
//! the concrete storage engine. [`SqliteCaseRepository`] is the only
//! implementation today.

mod error;
mod sqlite;

pub use error::RepositoryError;
pub use sqlite::SqliteCaseRepository;

use async_trait::async_trait;
use domain::ComplianceCase;
use uuid::Uuid;

#[async_trait]
pub trait CaseRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ComplianceCase>, RepositoryError>;
    async fn get(&self, id: Uuid) -> Result<Option<ComplianceCase>, RepositoryError>;
    /// Inserts a new case or replaces the existing one with the same id.
    async fn upsert(&self, case: &ComplianceCase) -> Result<(), RepositoryError>;
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
}
