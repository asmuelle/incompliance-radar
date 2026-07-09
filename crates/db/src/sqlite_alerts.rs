use crate::{AlertRepository, RepositoryError};
use async_trait::async_trait;
use domain::{Alert, WatchRule};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Shares its [`SqlitePool`] with a [`crate::SqliteCaseRepository`] (see
/// `SqliteCaseRepository::alert_repository`) rather than opening a second
/// connection to the same database file.
pub struct SqliteAlertRepository {
    pool: SqlitePool,
}

impl SqliteAlertRepository {
    pub(crate) fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn row_to_rule(row: &sqlx::sqlite::SqliteRow) -> Result<WatchRule, RepositoryError> {
        Ok(WatchRule {
            id: parse_uuid(row.try_get("id")?)?,
            label: row.try_get("label")?,
            industry: row.try_get("industry")?,
            company_name_contains: row.try_get("company_name_contains")?,
            regime: parse_enum_json(row.try_get("regime")?)?,
            regulator_slug: row.try_get("regulator_slug")?,
            violation_type: parse_enum_json(row.try_get("violation_type")?)?,
            created_at: parse_datetime(row.try_get("created_at")?)?,
        })
    }

    fn row_to_alert(row: &sqlx::sqlite::SqliteRow) -> Result<Alert, RepositoryError> {
        Ok(Alert {
            id: parse_uuid(row.try_get("id")?)?,
            watch_rule_id: parse_uuid(row.try_get("watch_rule_id")?)?,
            watch_rule_label: row.try_get("watch_rule_label")?,
            case_id: parse_uuid(row.try_get("case_id")?)?,
            company_name: row.try_get("company_name")?,
            message: row.try_get("message")?,
            created_at: parse_datetime(row.try_get("created_at")?)?,
            acknowledged: row.try_get::<i64, _>("acknowledged")? != 0,
        })
    }
}

fn parse_uuid(value: String) -> Result<Uuid, RepositoryError> {
    Uuid::parse_str(&value)
        .map_err(|e| RepositoryError::Decode(format!("invalid uuid '{value}': {e}")))
}

/// Enum-valued watch-rule criteria (`regime`, `violation_type`) are stored as
/// serde JSON text so `Other(..)` variants round-trip exactly; NULL means the
/// criterion isn't set.
fn parse_enum_json<T: serde::de::DeserializeOwned>(
    value: Option<String>,
) -> Result<Option<T>, RepositoryError> {
    value
        .map(|json| {
            serde_json::from_str(&json)
                .map_err(|e| RepositoryError::Decode(format!("invalid criterion '{json}': {e}")))
        })
        .transpose()
}

fn to_enum_json<T: serde::Serialize>(value: &Option<T>) -> Result<Option<String>, RepositoryError> {
    value
        .as_ref()
        .map(|v| {
            serde_json::to_string(v)
                .map_err(|e| RepositoryError::Decode(format!("unserializable criterion: {e}")))
        })
        .transpose()
}

fn parse_datetime(value: String) -> Result<chrono::NaiveDateTime, RepositoryError> {
    chrono::NaiveDateTime::parse_from_str(&value, DATETIME_FORMAT)
        .map_err(|e| RepositoryError::Decode(format!("invalid datetime '{value}': {e}")))
}

#[async_trait]
impl AlertRepository for SqliteAlertRepository {
    async fn list_rules(&self) -> Result<Vec<WatchRule>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM watch_rules ORDER BY created_at")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(Self::row_to_rule).collect()
    }

    async fn create_rule(&self, rule: &WatchRule) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO watch_rules
                (id, label, industry, company_name_contains,
                 regime, regulator_slug, violation_type, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(rule.id.to_string())
        .bind(&rule.label)
        .bind(&rule.industry)
        .bind(&rule.company_name_contains)
        .bind(to_enum_json(&rule.regime)?)
        .bind(&rule.regulator_slug)
        .bind(to_enum_json(&rule.violation_type)?)
        .bind(rule.created_at.format(DATETIME_FORMAT).to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_rule(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM watch_rules WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_alerts(&self) -> Result<Vec<Alert>, RepositoryError> {
        let rows = sqlx::query("SELECT * FROM alerts ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(Self::row_to_alert).collect()
    }

    async fn record_alert(&self, alert: &Alert) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO alerts
                (id, watch_rule_id, watch_rule_label, case_id, company_name, message, created_at, acknowledged)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(alert.id.to_string())
        .bind(alert.watch_rule_id.to_string())
        .bind(&alert.watch_rule_label)
        .bind(alert.case_id.to_string())
        .bind(&alert.company_name)
        .bind(&alert.message)
        .bind(alert.created_at.format(DATETIME_FORMAT).to_string())
        .bind(alert.acknowledged as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn acknowledge_alert(&self, id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE alerts SET acknowledged = 1 WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteCaseRepository;
    use domain::{Company, ComplianceCase};

    async fn in_memory_repo() -> SqliteAlertRepository {
        let case_repo = SqliteCaseRepository::connect("sqlite::memory:")
            .await
            .unwrap();
        case_repo.alert_repository()
    }

    fn now() -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(12, 30, 0)
            .unwrap()
    }

    fn demo_rule() -> WatchRule {
        WatchRule::new("Banking watch", Some("Banking".to_string()), None, now())
    }

    fn demo_case() -> ComplianceCase {
        ComplianceCase::new(Company::new("Acme Bank", "Banking", "US"))
    }

    #[tokio::test]
    async fn create_then_list_rules_roundtrips() {
        let repo = in_memory_repo().await;
        let rule = demo_rule();
        repo.create_rule(&rule).await.unwrap();

        let rules = repo.list_rules().await.unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, rule.id);
        assert_eq!(rules[0].label, "Banking watch");
        assert_eq!(rules[0].industry.as_deref(), Some("Banking"));
    }

    #[tokio::test]
    async fn resolution_criteria_roundtrip_including_other_variants() {
        let repo = in_memory_repo().await;
        let rule = WatchRule::new("Privacy watch", None, None, now())
            .with_regime(Some(domain::Regime::DataProtection))
            .with_regulator_slug(Some("ie-dpc".to_string()))
            .with_violation_type(Some(domain::ViolationType::Other(
                "Telemetry Overreach".to_string(),
            )));
        repo.create_rule(&rule).await.unwrap();

        let rules = repo.list_rules().await.unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0], rule);
    }

    #[tokio::test]
    async fn rules_without_resolution_criteria_load_with_none() {
        // Mirrors rows created before migration 0003 (columns NULL).
        let repo = in_memory_repo().await;
        repo.create_rule(&demo_rule()).await.unwrap();

        let rules = repo.list_rules().await.unwrap();

        assert_eq!(rules[0].regime, None);
        assert_eq!(rules[0].regulator_slug, None);
        assert_eq!(rules[0].violation_type, None);
    }

    #[tokio::test]
    async fn delete_rule_removes_it() {
        let repo = in_memory_repo().await;
        let rule = demo_rule();
        repo.create_rule(&rule).await.unwrap();

        repo.delete_rule(rule.id).await.unwrap();

        assert!(repo.list_rules().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn record_then_list_alerts_roundtrips() {
        let repo = in_memory_repo().await;
        let rule = demo_rule();
        let case = demo_case();
        let alert = Alert::new(&rule, &case, now());

        repo.record_alert(&alert).await.unwrap();
        let alerts = repo.list_alerts().await.unwrap();

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].id, alert.id);
        assert_eq!(alerts[0].company_name, "Acme Bank");
        assert!(!alerts[0].acknowledged);
    }

    #[tokio::test]
    async fn acknowledge_alert_marks_it_acknowledged() {
        let repo = in_memory_repo().await;
        let rule = demo_rule();
        let case = demo_case();
        let alert = Alert::new(&rule, &case, now());
        repo.record_alert(&alert).await.unwrap();

        repo.acknowledge_alert(alert.id).await.unwrap();

        let alerts = repo.list_alerts().await.unwrap();
        assert!(alerts[0].acknowledged);
    }

    #[tokio::test]
    async fn list_alerts_orders_newest_first() {
        let repo = in_memory_repo().await;
        let rule = demo_rule();
        let case = demo_case();
        let earlier = Alert::new(&rule, &case, now());
        let later = Alert::new(&rule, &case, now() + chrono::Duration::hours(1));

        repo.record_alert(&earlier).await.unwrap();
        repo.record_alert(&later).await.unwrap();

        let alerts = repo.list_alerts().await.unwrap();
        assert_eq!(alerts[0].id, later.id);
        assert_eq!(alerts[1].id, earlier.id);
    }
}
