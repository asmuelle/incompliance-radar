use crate::RepositoryError;
use async_trait::async_trait;
use domain::{Alert, ComplianceCase, WatchRule};
use uuid::Uuid;

#[async_trait]
pub trait AlertRepository: Send + Sync {
    async fn list_rules(&self) -> Result<Vec<WatchRule>, RepositoryError>;
    async fn create_rule(&self, rule: &WatchRule) -> Result<(), RepositoryError>;
    async fn delete_rule(&self, id: Uuid) -> Result<(), RepositoryError>;

    /// Newest first.
    async fn list_alerts(&self) -> Result<Vec<Alert>, RepositoryError>;
    async fn record_alert(&self, alert: &Alert) -> Result<(), RepositoryError>;
    async fn acknowledge_alert(&self, id: Uuid) -> Result<(), RepositoryError>;
}

/// Checks `case` against every rule in the repository and records (and
/// returns) an [`Alert`] for each match. Called after persisting a case —
/// both `extract_case` (`web/app/src/server_fns.rs`) and the crawler
/// (`crates/crawler`) call this the same way, so a case triggers alerts
/// regardless of whether it arrived via manual paste or an automated fetch.
pub async fn evaluate_case(
    case: &ComplianceCase,
    repo: &dyn AlertRepository,
) -> Result<Vec<Alert>, RepositoryError> {
    let rules = repo.list_rules().await?;
    let now = chrono::Utc::now().naive_utc();

    let mut triggered = Vec::new();
    for rule in rules.iter().filter(|rule| rule.matches(case)) {
        let alert = Alert::new(rule, case, now);
        repo.record_alert(&alert).await?;
        triggered.push(alert);
    }
    Ok(triggered)
}
