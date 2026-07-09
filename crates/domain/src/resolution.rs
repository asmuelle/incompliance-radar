use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{Monitor, Regime, Regulator, Sanction, ViolationType};

/// The legal instrument used to resolve the enforcement action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionKind {
    DeferredProsecutionAgreement,
    NonProsecutionAgreement,
    ConsentOrder,
    /// Standalone independent compliance monitorship, not tied to a DPA/NPA.
    Monitorship,
    /// Administrative fine imposed directly by a regulator (e.g. a GDPR
    /// Article 83 fine) — no negotiated agreement involved.
    AdministrativeFine,
    /// Negotiated civil settlement (e.g. FTC/state-AG consent settlements).
    Settlement,
    /// Civil (money) penalty order — OFAC penalty notices, CFPB CMPs.
    CivilPenalty,
    /// Cease-and-desist order (banking supervision's workhorse instrument).
    CeaseAndDesist,
    Other(String),
}

impl ResolutionKind {
    /// Maps free text (variant name, display label, or common shorthand —
    /// spacing/dashes/case ignored) onto a known variant, falling back to
    /// `Other` so nothing is dropped at a boundary.
    pub fn parse(value: &str) -> ResolutionKind {
        let normalized: String = value
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        match normalized.as_str() {
            "deferredprosecutionagreement" | "dpa" => ResolutionKind::DeferredProsecutionAgreement,
            "nonprosecutionagreement" | "npa" => ResolutionKind::NonProsecutionAgreement,
            "consentorder" => ResolutionKind::ConsentOrder,
            "monitorship" => ResolutionKind::Monitorship,
            "administrativefine" => ResolutionKind::AdministrativeFine,
            "settlement" => ResolutionKind::Settlement,
            "civilpenalty" | "civilmoneypenalty" => ResolutionKind::CivilPenalty,
            "ceaseanddesist" | "ceaseanddesistorder" => ResolutionKind::CeaseAndDesist,
            _ => ResolutionKind::Other(value.trim().to_string()),
        }
    }
}

impl fmt::Display for ResolutionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ResolutionKind::DeferredProsecutionAgreement => "DPA",
            ResolutionKind::NonProsecutionAgreement => "NPA",
            ResolutionKind::ConsentOrder => "Consent Order",
            ResolutionKind::Monitorship => "Monitorship",
            ResolutionKind::AdministrativeFine => "Administrative Fine",
            ResolutionKind::Settlement => "Settlement",
            ResolutionKind::CivilPenalty => "Civil Penalty",
            ResolutionKind::CeaseAndDesist => "Cease and Desist",
            ResolutionKind::Other(name) => name.as_str(),
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStatus {
    Active,
    Completed,
    Terminated,
    Breached,
}

impl fmt::Display for ResolutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ResolutionStatus::Active => "Active",
            ResolutionStatus::Completed => "Completed",
            ResolutionStatus::Terminated => "Terminated",
            ResolutionStatus::Breached => "Breached",
        };
        f.write_str(label)
    }
}

// (tests at the bottom of this file cover the legacy-JSON compatibility
// promises: pre-regime resolutions and enum-encoded regulators.)

/// A single enforcement resolution (DPA, NPA, monitorship, fine, consent
/// order, ...) extracted from a regulatory filing, press release or court
/// document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    /// `#[serde(default)]` maps every case persisted before regimes existed
    /// to `CorporateProsecution` — the only regime tracked back then — so
    /// old JSON blobs in SQLite deserialize without a migration.
    #[serde(default)]
    pub regime: Regime,
    pub regulator: Regulator,
    pub kind: ResolutionKind,
    pub status: ResolutionStatus,
    pub signed_on: Option<chrono::NaiveDate>,
    pub term_months: Option<u32>,
    pub monitor: Option<Monitor>,
    pub violations: Vec<ViolationType>,
    pub sanctions: Vec<Sanction>,
    pub obligations: Vec<String>,
    /// URL or citation for the primary source document.
    pub source: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Regime;

    #[test]
    fn legacy_resolution_json_deserializes_with_default_regime() {
        // The exact shape every case row persisted before the regime/registry
        // refactor: enum-tag regulator, no `regime` key. Stored JSON blobs in
        // SQLite are never rewritten, so this must keep working indefinitely.
        let json = r#"{
            "regulator": "Doj",
            "kind": "DeferredProsecutionAgreement",
            "status": "Active",
            "signed_on": "2024-03-15",
            "term_months": 36,
            "monitor": null,
            "violations": [{"Other": "Wire Fraud"}, "Bribery"],
            "sanctions": [],
            "obligations": [],
            "source": null
        }"#;

        let resolution: Resolution = serde_json::from_str(json).unwrap();

        assert_eq!(resolution.regime, Regime::CorporateProsecution);
        assert_eq!(resolution.regulator, Regulator::doj());
        assert_eq!(
            resolution.kind,
            ResolutionKind::DeferredProsecutionAgreement
        );
        assert_eq!(
            resolution.violations,
            vec![
                ViolationType::Other("Wire Fraud".to_string()),
                ViolationType::Bribery
            ]
        );
    }

    #[test]
    fn legacy_other_regulator_json_deserializes() {
        let json = r#"{
            "regulator": {"Other": "BaFin"},
            "kind": {"Other": "Administrative Order"},
            "status": "Completed",
            "signed_on": null,
            "term_months": null,
            "monitor": null,
            "violations": [],
            "sanctions": [],
            "obligations": [],
            "source": null
        }"#;

        let resolution: Resolution = serde_json::from_str(json).unwrap();

        assert_eq!(resolution.regulator, Regulator::other("BaFin"));
        assert_eq!(
            resolution.kind,
            ResolutionKind::Other("Administrative Order".to_string())
        );
    }

    #[test]
    fn current_resolution_roundtrips() {
        let resolution = Resolution {
            regime: Regime::DataProtection,
            regulator: Regulator::from_slug("ie-dpc").unwrap(),
            kind: ResolutionKind::AdministrativeFine,
            status: ResolutionStatus::Completed,
            signed_on: chrono::NaiveDate::from_ymd_opt(2026, 5, 1),
            term_months: None,
            monitor: None,
            violations: vec![ViolationType::DataProtection],
            sanctions: vec![],
            obligations: vec![],
            source: Some("https://example.com/decision".to_string()),
        };

        let json = serde_json::to_string(&resolution).unwrap();
        let parsed: Resolution = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, resolution);
    }
}
