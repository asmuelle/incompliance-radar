use crate::{CaseFilter, CaseRepository, RepositoryError, SqliteAlertRepository};
use async_trait::async_trait;
use domain::ComplianceCase;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// SQLite-backed [`CaseRepository`]. Stores each case as a JSON blob alongside
/// a few indexed columns (company name, industry, jurisdiction) for the
/// filtering the search UI will eventually need — see migration
/// `0001_create_compliance_cases.sql` for why the schema isn't fully
/// normalized yet.
pub struct SqliteCaseRepository {
    pool: SqlitePool,
}

impl SqliteCaseRepository {
    /// Connects to `database_url` (e.g. `sqlite://incompliance-radar.db?mode=rwc`
    /// to create the file if missing, or `sqlite::memory:` for tests) and applies
    /// any pending migrations. A single connection is used deliberately — SQLite
    /// serializes writes anyway, and this avoids "database is locked" errors and,
    /// for `:memory:` URLs, accidentally opening multiple independent databases.
    pub async fn connect(database_url: &str) -> Result<Self, RepositoryError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    fn row_to_case(row: &sqlx::sqlite::SqliteRow) -> Result<ComplianceCase, RepositoryError> {
        let data: String = row.try_get("data")?;
        Ok(serde_json::from_str(&data)?)
    }

    /// Builds an [`AlertRepository`](crate::AlertRepository) sharing this
    /// repository's connection pool rather than opening a second connection
    /// to the same database file — migrations (including the watch_rules/
    /// alerts tables) already ran during `connect`.
    pub fn alert_repository(&self) -> SqliteAlertRepository {
        SqliteAlertRepository::new(self.pool.clone())
    }
}

#[async_trait]
impl CaseRepository for SqliteCaseRepository {
    async fn list(&self) -> Result<Vec<ComplianceCase>, RepositoryError> {
        let rows = sqlx::query("SELECT data FROM compliance_cases ORDER BY company_name")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(Self::row_to_case).collect()
    }

    async fn get(&self, id: Uuid) -> Result<Option<ComplianceCase>, RepositoryError> {
        let row = sqlx::query("SELECT data FROM compliance_cases WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(Self::row_to_case).transpose()
    }

    async fn upsert(&self, case: &ComplianceCase) -> Result<(), RepositoryError> {
        let data = serde_json::to_string(case)?;
        sqlx::query(
            "INSERT INTO compliance_cases (id, company_name, industry, jurisdiction, data, updated_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                company_name = excluded.company_name,
                industry = excluded.industry,
                jurisdiction = excluded.jurisdiction,
                data = excluded.data,
                updated_at = datetime('now')",
        )
        .bind(case.id.to_string())
        .bind(&case.company.name)
        .bind(&case.company.industry)
        .bind(&case.company.jurisdiction)
        .bind(data)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM compliance_cases WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// `industry`/`jurisdiction` are pushed down into SQL (indexed, exact
    /// match, case-insensitive via `COLLATE NOCASE`). `violation_type` and
    /// `monitor_firm` live inside each case's serialized resolutions, not as
    /// their own columns — normalizing those into indexed columns/junction
    /// tables is only worth it once the case count is large enough for an
    /// in-memory filter over the (already industry/jurisdiction-narrowed)
    /// result set to matter; see `0001_create_compliance_cases.sql`.
    async fn search(&self, filter: &CaseFilter) -> Result<Vec<ComplianceCase>, RepositoryError> {
        let mut query = String::from("SELECT data FROM compliance_cases WHERE 1 = 1");
        if filter.industry.is_some() {
            query.push_str(" AND industry = ? COLLATE NOCASE");
        }
        if filter.jurisdiction.is_some() {
            query.push_str(" AND jurisdiction = ? COLLATE NOCASE");
        }
        query.push_str(" ORDER BY company_name");

        let mut q = sqlx::query(&query);
        if let Some(industry) = &filter.industry {
            q = q.bind(industry);
        }
        if let Some(jurisdiction) = &filter.jurisdiction {
            q = q.bind(jurisdiction);
        }

        let rows = q.fetch_all(&self.pool).await?;
        let cases = rows
            .iter()
            .map(Self::row_to_case)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(cases
            .into_iter()
            .filter(|case| matches_case(case, filter))
            .collect())
    }
}

fn matches_case(case: &ComplianceCase, filter: &CaseFilter) -> bool {
    let violation_matches = |wanted: &str| {
        case.resolutions.iter().any(|r| {
            r.violations
                .iter()
                .any(|v| v.to_string().eq_ignore_ascii_case(wanted))
        })
    };
    let monitor_firm_matches = |wanted: &str| {
        case.resolutions.iter().any(|r| {
            r.monitor
                .as_ref()
                .and_then(|m| m.firm.as_deref())
                .is_some_and(|firm| firm.to_lowercase().contains(&wanted.to_lowercase()))
        })
    };

    filter
        .violation_type
        .as_deref()
        .is_none_or(violation_matches)
        && filter
            .monitor_firm
            .as_deref()
            .is_none_or(monitor_firm_matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::Company;

    async fn in_memory_repo() -> SqliteCaseRepository {
        SqliteCaseRepository::connect("sqlite::memory:")
            .await
            .unwrap()
    }

    fn demo_case() -> ComplianceCase {
        ComplianceCase::new(Company::new("Acme", "Manufacturing", "US"))
    }

    #[tokio::test]
    async fn upsert_then_list_roundtrips_case() {
        let repo = in_memory_repo().await;
        let case = demo_case();
        repo.upsert(&case).await.unwrap();

        let cases = repo.list().await.unwrap();

        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, case.id);
        assert_eq!(cases[0].company.name, "Acme");
    }

    #[tokio::test]
    async fn get_returns_none_for_unknown_id() {
        let repo = in_memory_repo().await;
        assert!(repo.get(Uuid::new_v4()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_returns_case_by_id() {
        let repo = in_memory_repo().await;
        let case = demo_case();
        repo.upsert(&case).await.unwrap();

        let found = repo.get(case.id).await.unwrap();

        assert_eq!(found.map(|c| c.id), Some(case.id));
    }

    #[tokio::test]
    async fn upsert_twice_updates_existing_row_instead_of_duplicating() {
        let repo = in_memory_repo().await;
        let mut case = demo_case();
        repo.upsert(&case).await.unwrap();

        case.company.name = "Acme Renamed".into();
        repo.upsert(&case).await.unwrap();

        let cases = repo.list().await.unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].company.name, "Acme Renamed");
    }

    #[tokio::test]
    async fn delete_removes_case() {
        let repo = in_memory_repo().await;
        let case = demo_case();
        repo.upsert(&case).await.unwrap();

        repo.delete(case.id).await.unwrap();

        assert!(repo.list().await.unwrap().is_empty());
    }

    fn case_with(
        name: &str,
        industry: &str,
        jurisdiction: &str,
        violation: domain::ViolationType,
        monitor_firm: Option<&str>,
    ) -> ComplianceCase {
        let mut case = ComplianceCase::new(Company::new(name, industry, jurisdiction));
        case.resolutions.push(domain::Resolution {
            regulator: domain::Regulator::Sec,
            kind: domain::ResolutionKind::ConsentOrder,
            status: domain::ResolutionStatus::Active,
            signed_on: None,
            term_months: None,
            monitor: monitor_firm.map(|firm| domain::Monitor {
                name: "Some Monitor".into(),
                firm: Some(firm.to_string()),
                appointed_on: None,
                term_months: None,
            }),
            violations: vec![violation],
            sanctions: Vec::new(),
            obligations: Vec::new(),
            source: None,
        });
        case
    }

    async fn seeded_repo() -> SqliteCaseRepository {
        let repo = in_memory_repo().await;
        repo.upsert(&case_with(
            "Acme Bank",
            "Banking",
            "US",
            domain::ViolationType::MoneyLaundering,
            Some("Kroll Compliance Partners"),
        ))
        .await
        .unwrap();
        repo.upsert(&case_with(
            "Widget Manufacturing",
            "Manufacturing",
            "UK",
            domain::ViolationType::Bribery,
            None,
        ))
        .await
        .unwrap();
        repo
    }

    #[tokio::test]
    async fn empty_filter_returns_everything() {
        let repo = seeded_repo().await;
        let results = repo.search(&CaseFilter::default()).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn filters_by_industry_case_insensitively() {
        let repo = seeded_repo().await;
        let filter = CaseFilter {
            industry: Some("banking".to_string()),
            ..Default::default()
        };

        let results = repo.search(&filter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].company.name, "Acme Bank");
    }

    #[tokio::test]
    async fn filters_by_jurisdiction() {
        let repo = seeded_repo().await;
        let filter = CaseFilter {
            jurisdiction: Some("UK".to_string()),
            ..Default::default()
        };

        let results = repo.search(&filter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].company.name, "Widget Manufacturing");
    }

    #[tokio::test]
    async fn filters_by_violation_type() {
        let repo = seeded_repo().await;
        let filter = CaseFilter {
            violation_type: Some("Money Laundering".to_string()),
            ..Default::default()
        };

        let results = repo.search(&filter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].company.name, "Acme Bank");
    }

    #[tokio::test]
    async fn filters_by_monitor_firm_substring_case_insensitively() {
        let repo = seeded_repo().await;
        let filter = CaseFilter {
            monitor_firm: Some("kroll".to_string()),
            ..Default::default()
        };

        let results = repo.search(&filter).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].company.name, "Acme Bank");
    }

    #[tokio::test]
    async fn combined_filters_are_anded() {
        let repo = seeded_repo().await;
        let filter = CaseFilter {
            industry: Some("Banking".to_string()),
            violation_type: Some("Bribery".to_string()),
            ..Default::default()
        };

        let results = repo.search(&filter).await.unwrap();

        assert!(
            results.is_empty(),
            "Acme Bank's violation is MoneyLaundering, not Bribery"
        );
    }

    #[tokio::test]
    async fn no_matches_returns_empty_vec() {
        let repo = seeded_repo().await;
        let filter = CaseFilter {
            industry: Some("Aerospace".to_string()),
            ..Default::default()
        };

        assert!(repo.search(&filter).await.unwrap().is_empty());
    }
}
