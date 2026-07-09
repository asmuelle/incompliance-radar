use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ComplianceCase, Regime, ViolationType};

/// A global watch rule: fire when a case matches every criterion set (AND).
/// At least one criterion should be set for a rule to ever match anything,
/// but that's a UI-level nicety, not enforced here — an all-`None` rule
/// simply never matches (see `matches`).
///
/// Company-level criteria (`industry`, `company_name_contains`) match against
/// the case's company; resolution-level criteria (`regime`, `regulator_slug`,
/// `violation_type`) must all hold on a *single* resolution — "a
/// data-protection fine from the DPC" means one resolution that is both, not
/// a DPC resolution plus an unrelated data-protection one.
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
    /// Enforcement regime a resolution must belong to (exact match).
    /// `#[serde(default)]` keeps rules persisted before regimes deserializing.
    #[serde(default)]
    pub regime: Option<Regime>,
    /// Canonical regulator slug (see `Regulator::slug`), matched
    /// case-insensitively against a resolution's regulator.
    #[serde(default)]
    pub regulator_slug: Option<String>,
    /// Violation a resolution must include (exact variant match).
    #[serde(default)]
    pub violation_type: Option<ViolationType>,
    pub created_at: chrono::NaiveDateTime,
}

impl WatchRule {
    /// The two original company-level criteria stay positional (they're the
    /// common case); resolution-level criteria are added via the `with_*`
    /// builders below.
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
            regime: None,
            regulator_slug: None,
            violation_type: None,
            created_at,
        }
    }

    pub fn with_regime(self, regime: Option<Regime>) -> Self {
        Self { regime, ..self }
    }

    pub fn with_regulator_slug(self, regulator_slug: Option<String>) -> Self {
        Self {
            regulator_slug,
            ..self
        }
    }

    pub fn with_violation_type(self, violation_type: Option<ViolationType>) -> Self {
        Self {
            violation_type,
            ..self
        }
    }

    /// True if `case` satisfies every criterion this rule sets. A rule with
    /// no criteria at all never matches — it's not "match everything".
    pub fn matches(&self, case: &ComplianceCase) -> bool {
        let has_resolution_criteria =
            self.regime.is_some() || self.regulator_slug.is_some() || self.violation_type.is_some();
        if self.industry.is_none()
            && self.company_name_contains.is_none()
            && !has_resolution_criteria
        {
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
        let resolution_matches = !has_resolution_criteria
            || case.resolutions.iter().any(|resolution| {
                let regime_ok = self
                    .regime
                    .as_ref()
                    .is_none_or(|wanted| &resolution.regime == wanted);
                let regulator_ok = self
                    .regulator_slug
                    .as_deref()
                    .is_none_or(|wanted| resolution.regulator.slug.eq_ignore_ascii_case(wanted));
                let violation_ok = self
                    .violation_type
                    .as_ref()
                    .is_none_or(|wanted| resolution.violations.contains(wanted));
                regime_ok && regulator_ok && violation_ok
            });

        industry_matches && company_matches && resolution_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Company, Regulator, Resolution, ResolutionKind, ResolutionStatus};

    fn case_for(name: &str, industry: &str) -> ComplianceCase {
        ComplianceCase::new(Company::new(name, industry, "US"))
    }

    fn resolution(
        regime: Regime,
        regulator: Regulator,
        violations: Vec<ViolationType>,
    ) -> Resolution {
        Resolution {
            regime,
            regulator,
            kind: ResolutionKind::AdministrativeFine,
            status: ResolutionStatus::Completed,
            signed_on: None,
            term_months: None,
            monitor: None,
            violations,
            sanctions: Vec::new(),
            obligations: Vec::new(),
            source: None,
        }
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

    #[test]
    fn matches_by_regime() {
        let rule = WatchRule::new("Privacy watch", None, None, now())
            .with_regime(Some(Regime::DataProtection));

        let mut privacy_case = case_for("Acme", "Ad Tech");
        privacy_case.resolutions.push(resolution(
            Regime::DataProtection,
            Regulator::from_slug("ie-dpc").unwrap(),
            vec![ViolationType::DataProtection],
        ));
        assert!(rule.matches(&privacy_case));

        let mut prosecution_case = case_for("Widget", "Ad Tech");
        prosecution_case.resolutions.push(resolution(
            Regime::CorporateProsecution,
            Regulator::doj(),
            vec![ViolationType::Bribery],
        ));
        assert!(!rule.matches(&prosecution_case));
    }

    #[test]
    fn matches_by_regulator_slug_case_insensitively() {
        let rule = WatchRule::new("DPC watch", None, None, now())
            .with_regulator_slug(Some("IE-DPC".to_string()));

        let mut dpc_case = case_for("Acme", "Ad Tech");
        dpc_case.resolutions.push(resolution(
            Regime::DataProtection,
            Regulator::from_slug("ie-dpc").unwrap(),
            vec![],
        ));
        assert!(rule.matches(&dpc_case));

        let mut doj_case = case_for("Acme", "Ad Tech");
        doj_case.resolutions.push(resolution(
            Regime::CorporateProsecution,
            Regulator::doj(),
            vec![],
        ));
        assert!(!rule.matches(&doj_case));
    }

    #[test]
    fn matches_by_violation_type() {
        let rule = WatchRule::new("Bribery watch", None, None, now())
            .with_violation_type(Some(ViolationType::Bribery));

        let mut bribery_case = case_for("Acme", "Energy");
        bribery_case.resolutions.push(resolution(
            Regime::CorporateProsecution,
            Regulator::doj(),
            vec![ViolationType::Bribery, ViolationType::MoneyLaundering],
        ));
        assert!(rule.matches(&bribery_case));

        let mut tax_case = case_for("Widget", "Energy");
        tax_case.resolutions.push(resolution(
            Regime::CorporateProsecution,
            Regulator::doj(),
            vec![ViolationType::TaxEvasion],
        ));
        assert!(!rule.matches(&tax_case));
    }

    #[test]
    fn resolution_criteria_must_hold_on_a_single_resolution() {
        // A DPC resolution without the violation plus a DoJ resolution with
        // it must NOT satisfy "DPC + bribery" — the criteria describe one
        // resolution, not the case as a whole.
        let rule = WatchRule::new("DPC bribery watch", None, None, now())
            .with_regulator_slug(Some("ie-dpc".to_string()))
            .with_violation_type(Some(ViolationType::Bribery));

        let mut split_case = case_for("Acme", "Ad Tech");
        split_case.resolutions.push(resolution(
            Regime::DataProtection,
            Regulator::from_slug("ie-dpc").unwrap(),
            vec![ViolationType::DataProtection],
        ));
        split_case.resolutions.push(resolution(
            Regime::CorporateProsecution,
            Regulator::doj(),
            vec![ViolationType::Bribery],
        ));
        assert!(!rule.matches(&split_case));

        let mut combined_case = case_for("Acme", "Ad Tech");
        combined_case.resolutions.push(resolution(
            Regime::DataProtection,
            Regulator::from_slug("ie-dpc").unwrap(),
            vec![ViolationType::Bribery],
        ));
        assert!(rule.matches(&combined_case));
    }

    #[test]
    fn company_and_resolution_criteria_combine_with_and() {
        let rule = WatchRule::new(
            "Pharma privacy watch",
            Some("Pharma".to_string()),
            None,
            now(),
        )
        .with_regime(Some(Regime::DataProtection));

        let mut matching = case_for("Acme", "Pharma");
        matching.resolutions.push(resolution(
            Regime::DataProtection,
            Regulator::from_slug("uk-ico").unwrap(),
            vec![],
        ));
        assert!(rule.matches(&matching));

        let mut wrong_industry = case_for("Widget", "Banking");
        wrong_industry.resolutions.push(resolution(
            Regime::DataProtection,
            Regulator::from_slug("uk-ico").unwrap(),
            vec![],
        ));
        assert!(!rule.matches(&wrong_industry));
    }

    #[test]
    fn resolution_criteria_never_match_a_case_with_no_resolutions() {
        let rule = WatchRule::new("Privacy watch", None, None, now())
            .with_regime(Some(Regime::DataProtection));
        assert!(!rule.matches(&case_for("Acme", "Ad Tech")));
    }

    #[test]
    fn pre_regime_rule_json_still_deserializes() {
        // Shape persisted by the app before resolution-level criteria
        // existed — must keep loading (serde defaults the new fields).
        let json = r#"{
            "id": "6f6c1a5e-3f6a-4c46-9d3e-2a1b7c8d9e0f",
            "label": "Banking watch",
            "industry": "Banking",
            "company_name_contains": null,
            "created_at": "2026-01-01T00:00:00"
        }"#;
        let rule: WatchRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.regime, None);
        assert_eq!(rule.regulator_slug, None);
        assert_eq!(rule.violation_type, None);
        assert!(rule.matches(&case_for("Acme Bank", "Banking")));
    }
}
