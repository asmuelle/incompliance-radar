use crate::ComplianceCase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountEntry {
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateEntry {
    pub label: String,
    /// Fraction in `[0.0, 1.0]` — e.g. the share of a given industry's
    /// resolutions that had a monitor appointed.
    pub rate: f64,
    /// Denominator `rate` was computed from, so callers can judge how much
    /// to trust a rate from a tiny sample (e.g. "100%" off of 1 resolution).
    pub sample_size: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmountEntry {
    pub currency: String,
    pub total: f64,
}

/// Aggregate statistics across every tracked case — "which industries see
/// the most monitors, which violation types dominate" per spec.md's trend/
/// benchmark analysis goal. Every `Vec` is sorted descending by its primary
/// metric (count/rate/total), with `label` ascending as a deterministic
/// tiebreaker (so output — and therefore tests — don't depend on
/// `HashMap` iteration order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendReport {
    pub total_cases: usize,
    pub cases_by_industry: Vec<CountEntry>,
    pub resolutions_by_regulator: Vec<CountEntry>,
    pub resolutions_by_violation_type: Vec<CountEntry>,
    pub resolutions_by_kind: Vec<CountEntry>,
    pub resolutions_by_status: Vec<CountEntry>,
    /// Share of each industry's resolutions with a monitor appointed —
    /// spec.md's "which industries does the DoJ currently favor monitors
    /// in" made concrete across whichever regulators are tracked.
    pub monitorship_rate_by_industry: Vec<RateEntry>,
    /// Summed separately per currency — deliberately not converted to a
    /// single currency, which would require a live FX rate this app has no
    /// reason to depend on.
    pub total_sanctions_by_currency: Vec<AmountEntry>,
}

pub fn compute_trend_report(cases: &[ComplianceCase]) -> TrendReport {
    let mut cases_by_industry = Counter::new();
    let mut resolutions_by_regulator = Counter::new();
    let mut resolutions_by_violation_type = Counter::new();
    let mut resolutions_by_kind = Counter::new();
    let mut resolutions_by_status = Counter::new();
    let mut monitorship: HashMap<String, (usize, usize)> = HashMap::new();
    let mut sanctions_by_currency: HashMap<String, f64> = HashMap::new();

    for case in cases {
        cases_by_industry.add(&case.company.industry);

        let (with_monitor, total) = monitorship
            .entry(case.company.industry.clone())
            .or_default();
        *total += case.resolutions.len();
        *with_monitor += case
            .resolutions
            .iter()
            .filter(|r| r.monitor.is_some())
            .count();

        for resolution in &case.resolutions {
            resolutions_by_regulator.add(&resolution.regulator.to_string());
            resolutions_by_kind.add(&resolution.kind.to_string());
            resolutions_by_status.add(&resolution.status.to_string());
            for violation in &resolution.violations {
                resolutions_by_violation_type.add(&violation.to_string());
            }
            for sanction in &resolution.sanctions {
                *sanctions_by_currency
                    .entry(sanction.currency.clone())
                    .or_default() += sanction.amount;
            }
        }
    }

    let mut monitorship_rate_by_industry: Vec<RateEntry> = monitorship
        .into_iter()
        .filter(|(_, (_, total))| *total > 0)
        .map(|(label, (with_monitor, total))| RateEntry {
            label,
            rate: with_monitor as f64 / total as f64,
            sample_size: total,
        })
        .collect();
    monitorship_rate_by_industry.sort_by(|a, b| {
        b.rate
            .partial_cmp(&a.rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.label.cmp(&b.label))
    });

    let mut total_sanctions_by_currency: Vec<AmountEntry> = sanctions_by_currency
        .into_iter()
        .map(|(currency, total)| AmountEntry { currency, total })
        .collect();
    total_sanctions_by_currency.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.currency.cmp(&b.currency))
    });

    TrendReport {
        total_cases: cases.len(),
        cases_by_industry: cases_by_industry.into_sorted_entries(),
        resolutions_by_regulator: resolutions_by_regulator.into_sorted_entries(),
        resolutions_by_violation_type: resolutions_by_violation_type.into_sorted_entries(),
        resolutions_by_kind: resolutions_by_kind.into_sorted_entries(),
        resolutions_by_status: resolutions_by_status.into_sorted_entries(),
        monitorship_rate_by_industry,
        total_sanctions_by_currency,
    }
}

#[derive(Default)]
struct Counter(HashMap<String, usize>);

impl Counter {
    fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, label: &str) {
        *self.0.entry(label.to_string()).or_default() += 1;
    }

    fn into_sorted_entries(self) -> Vec<CountEntry> {
        let mut entries: Vec<CountEntry> = self
            .0
            .into_iter()
            .map(|(label, count)| CountEntry { label, count })
            .collect();
        entries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Company, Monitor, Regulator, Resolution, ResolutionKind, ResolutionStatus, Sanction,
        ViolationType,
    };

    fn resolution(
        regulator: Regulator,
        kind: ResolutionKind,
        status: ResolutionStatus,
        violations: Vec<ViolationType>,
        monitor: Option<Monitor>,
        sanctions: Vec<Sanction>,
    ) -> Resolution {
        Resolution {
            regime: regulator.regime.clone().unwrap_or_default(),
            regulator,
            kind,
            status,
            signed_on: None,
            term_months: None,
            monitor,
            violations,
            sanctions,
            obligations: Vec::new(),
            source: None,
        }
    }

    fn monitor(name: &str) -> Monitor {
        Monitor {
            name: name.to_string(),
            firm: None,
            appointed_on: None,
            term_months: None,
        }
    }

    #[test]
    fn empty_input_yields_empty_report() {
        let report = compute_trend_report(&[]);

        assert_eq!(report.total_cases, 0);
        assert!(report.cases_by_industry.is_empty());
        assert!(report.monitorship_rate_by_industry.is_empty());
        assert!(report.total_sanctions_by_currency.is_empty());
    }

    #[test]
    fn counts_cases_by_industry() {
        let mut acme = ComplianceCase::new(Company::new("Acme", "Banking", "US"));
        acme.resolutions.push(resolution(
            Regulator::sec(),
            ResolutionKind::ConsentOrder,
            ResolutionStatus::Completed,
            vec![ViolationType::MoneyLaundering],
            None,
            vec![],
        ));
        let mut widget = ComplianceCase::new(Company::new("Widget Corp", "Banking", "UK"));
        widget.resolutions.push(resolution(
            Regulator::fca(),
            ResolutionKind::ConsentOrder,
            ResolutionStatus::Active,
            vec![ViolationType::Bribery],
            None,
            vec![],
        ));
        let manufacturing = ComplianceCase::new(Company::new("Steel Co", "Manufacturing", "US"));

        let report = compute_trend_report(&[acme, widget, manufacturing]);

        assert_eq!(report.total_cases, 3);
        assert_eq!(
            report.cases_by_industry[0],
            CountEntry {
                label: "Banking".to_string(),
                count: 2
            }
        );
        assert_eq!(
            report.cases_by_industry[1],
            CountEntry {
                label: "Manufacturing".to_string(),
                count: 1
            }
        );
    }

    #[test]
    fn monitorship_rate_reflects_share_of_resolutions_with_a_monitor() {
        let mut banking_with_monitor = ComplianceCase::new(Company::new("Acme", "Banking", "US"));
        banking_with_monitor.resolutions.push(resolution(
            Regulator::doj(),
            ResolutionKind::DeferredProsecutionAgreement,
            ResolutionStatus::Active,
            vec![],
            Some(monitor("Jane Doe")),
            vec![],
        ));
        let mut banking_without_monitor =
            ComplianceCase::new(Company::new("Widget", "Banking", "US"));
        banking_without_monitor.resolutions.push(resolution(
            Regulator::sec(),
            ResolutionKind::ConsentOrder,
            ResolutionStatus::Completed,
            vec![],
            None,
            vec![],
        ));

        let report = compute_trend_report(&[banking_with_monitor, banking_without_monitor]);

        let banking = report
            .monitorship_rate_by_industry
            .iter()
            .find(|e| e.label == "Banking")
            .unwrap();
        assert_eq!(banking.rate, 0.5);
        assert_eq!(banking.sample_size, 2);
    }

    #[test]
    fn sums_sanctions_per_currency_without_converting() {
        let mut usd_case = ComplianceCase::new(Company::new("Acme", "Banking", "US"));
        usd_case.resolutions.push(resolution(
            Regulator::sec(),
            ResolutionKind::ConsentOrder,
            ResolutionStatus::Completed,
            vec![],
            None,
            vec![
                Sanction {
                    amount: 1_000_000.0,
                    currency: "USD".to_string(),
                    description: None,
                },
                Sanction {
                    amount: 500_000.0,
                    currency: "USD".to_string(),
                    description: None,
                },
            ],
        ));
        let mut gbp_case = ComplianceCase::new(Company::new("Widget", "Banking", "UK"));
        gbp_case.resolutions.push(resolution(
            Regulator::fca(),
            ResolutionKind::ConsentOrder,
            ResolutionStatus::Completed,
            vec![],
            None,
            vec![Sanction {
                amount: 2_000_000.0,
                currency: "GBP".to_string(),
                description: None,
            }],
        ));

        let report = compute_trend_report(&[usd_case, gbp_case]);

        assert_eq!(
            report.total_sanctions_by_currency,
            vec![
                AmountEntry {
                    currency: "GBP".to_string(),
                    total: 2_000_000.0
                },
                AmountEntry {
                    currency: "USD".to_string(),
                    total: 1_500_000.0
                },
            ]
        );
    }

    #[test]
    fn ties_broken_deterministically_by_label() {
        let banking = ComplianceCase::new(Company::new("Acme", "Banking", "US"));
        let manufacturing = ComplianceCase::new(Company::new("Steel Co", "Manufacturing", "US"));

        let report = compute_trend_report(&[banking, manufacturing]);

        // Both have count 1 — alphabetical tiebreak makes this deterministic
        // across runs despite HashMap's unspecified iteration order.
        assert_eq!(report.cases_by_industry[0].label, "Banking");
        assert_eq!(report.cases_by_industry[1].label, "Manufacturing");
    }
}
