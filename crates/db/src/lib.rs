//! Persistence for compliance cases, behind the [`CaseRepository`] trait so
//! callers (server functions, future crawler/extraction jobs) never depend on
//! the concrete storage engine. [`SqliteCaseRepository`] is the only
//! implementation today.

mod alert_repository;
mod error;
mod filter;
mod sqlite;
mod sqlite_alerts;

pub use alert_repository::{evaluate_case, AlertRepository};
pub use error::RepositoryError;
pub use filter::CaseFilter;
pub use sqlite::SqliteCaseRepository;
pub use sqlite_alerts::SqliteAlertRepository;

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
    /// Cases matching every criterion set in `filter` (AND). An empty filter
    /// behaves like `list()`.
    async fn search(&self, filter: &CaseFilter) -> Result<Vec<ComplianceCase>, RepositoryError>;
}
