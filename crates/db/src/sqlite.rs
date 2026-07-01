use crate::{CaseRepository, RepositoryError};
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
}
