use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ComplianceCase;

/// A global watch rule: fire when a case matches every criterion set (AND).
/// At least one of `industry`/`company_name_contains` should be set for a
/// rule to ever match anything, but that's a UI-level nicety, not enforced
/// here — an all-`None` rule simply never matches (see `matches`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchRule {
    pub id: Uuid,
    /// Human label shown in the UI and copied onto any `Alert` it triggers,
    /// e.g. "Banking industry" or "Acme Corp".
    pub label: String,
    pub industry: Option<String>,
    /// Case-insensitive substring match against the company name — this is
    /// how "watch a competitor" is expressed without a dedicated company
    /// registry.
    pub company_name_contains: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

impl WatchRule {
    pub fn new(
        label: impl Into<String>,
        industry: Option<String>,
        company_name_contains: Option<String>,
        created_at: chrono::NaiveDateTime,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: label.into(),
            industry,
            company_name_contains,
            created_at,
        }
    }

    /// True if `case` satisfies every criterion this rule sets. A rule with
    /// no criteria at all never matches — it's not "match everything".
    pub fn matches(&self, case: &ComplianceCase) -> bool {
        if self.industry.is_none() && self.company_name_contains.is_none() {
            return false;
        }

        let industry_matches = self
            .industry
            .as_deref()
            .is_none_or(|wanted| case.company.industry.eq_ignore_ascii_case(wanted));
        let company_matches = self.company_name_contains.as_deref().is_none_or(|wanted| {
            case.company
                .name
                .to_lowercase()
                .contains(&wanted.to_lowercase())
        });

        industry_matches && company_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Company;

    fn case_for(name: &str, industry: &str) -> ComplianceCase {
        ComplianceCase::new(Company::new(name, industry, "US"))
    }

    fn now() -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    #[test]
    fn rule_with_no_criteria_never_matches() {
        let rule = WatchRule::new("Empty rule", None, None, now());
        assert!(!rule.matches(&case_for("Acme", "Banking")));
    }

    #[test]
    fn matches_by_industry_case_insensitively() {
        let rule = WatchRule::new("Banking watch", Some("banking".to_string()), None, now());
        assert!(rule.matches(&case_for("Acme", "Banking")));
        assert!(!rule.matches(&case_for("Acme", "Manufacturing")));
    }

    #[test]
    fn matches_by_company_name_substring_case_insensitively() {
        let rule = WatchRule::new("Competitor watch", None, Some("acme".to_string()), now());
        assert!(rule.matches(&case_for("Acme Global Industries", "Banking")));
        assert!(!rule.matches(&case_for("Widget Corp", "Banking")));
    }

    #[test]
    fn combined_criteria_are_anded() {
        let rule = WatchRule::new(
            "Acme in banking",
            Some("Banking".to_string()),
            Some("Acme".to_string()),
            now(),
        );
        assert!(rule.matches(&case_for("Acme Global Industries", "Banking")));
        assert!(!rule.matches(&case_for("Acme Global Industries", "Manufacturing")));
    }
}
