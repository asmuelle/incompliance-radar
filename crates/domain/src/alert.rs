use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ComplianceCase, WatchRule};

/// A recorded match between a [`WatchRule`] and a [`ComplianceCase`].
/// `watch_rule_label`/`company_name` are copied at creation time rather than
/// looked up via `watch_rule_id`/`case_id` on every read — the alert feed
/// should stay readable even if the rule or case it referenced is later
/// deleted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Alert {
    pub id: Uuid,
    pub watch_rule_id: Uuid,
    pub watch_rule_label: String,
    pub case_id: Uuid,
    pub company_name: String,
    pub message: String,
    pub created_at: chrono::NaiveDateTime,
    pub acknowledged: bool,
}

impl Alert {
    /// Builds the alert that fires when `rule` matches `case` — callers are
    /// expected to have already confirmed `rule.matches(case)`.
    pub fn new(rule: &WatchRule, case: &ComplianceCase, created_at: chrono::NaiveDateTime) -> Self {
        Self {
            id: Uuid::new_v4(),
            watch_rule_id: rule.id,
            watch_rule_label: rule.label.clone(),
            case_id: case.id,
            company_name: case.company.name.clone(),
            message: format!(
                "\"{}\" matches watch rule \"{}\"",
                case.company.name, rule.label
            ),
            created_at,
            acknowledged: false,
        }
    }
}
