use serde::{Deserialize, Serialize};
use std::fmt;

/// The enforcement domain a resolution belongs to: which body of law and
/// which kind of regulator produced it. Making this first-class (rather than
/// leaving it implicit in the regulator) is what lets search facets, watch
/// rules and trend reports slice across regimes — "any data-protection fine
/// in Pharma" — as coverage grows beyond DPAs/NPAs.
///
/// `#[serde(default)]` on `Resolution::regime` maps every pre-regime stored
/// case to `CorporateProsecution` (the only regime that existed then), so no
/// data migration is needed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Regime {
    /// Criminal corporate prosecution — DPAs, NPAs, pleas (DoJ, SFO).
    #[default]
    CorporateProsecution,
    /// Securities and market-conduct enforcement (SEC, FCA).
    SecuritiesEnforcement,
    /// Prudential banking supervision — consent orders, C&Ds (Fed, OCC, FDIC).
    BankingSupervision,
    /// Data-protection fines and orders (GDPR DPAs, ICO, state privacy laws).
    DataProtection,
    /// Economic-sanctions enforcement (OFAC civil penalties).
    SanctionsEnforcement,
    /// Consumer-protection enforcement (FTC, CFPB, state AGs).
    ConsumerProtection,
    Other(String),
}

impl Regime {
    /// Maps free text (a display label, variant name, or common shorthand,
    /// case-insensitive) onto a known variant, falling back to `Other` so
    /// nothing is silently dropped at a boundary.
    pub fn parse(value: &str) -> Regime {
        let normalized: String = value
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        match normalized.as_str() {
            "corporateprosecution" => Regime::CorporateProsecution,
            "securitiesenforcement" | "securities" => Regime::SecuritiesEnforcement,
            "bankingsupervision" | "banking" => Regime::BankingSupervision,
            "dataprotection" | "gdpr" | "privacy" => Regime::DataProtection,
            "sanctionsenforcement" | "sanctions" => Regime::SanctionsEnforcement,
            "consumerprotection" | "consumer" => Regime::ConsumerProtection,
            _ => Regime::Other(value.trim().to_string()),
        }
    }
}

impl fmt::Display for Regime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Regime::CorporateProsecution => "Corporate Prosecution",
            Regime::SecuritiesEnforcement => "Securities Enforcement",
            Regime::BankingSupervision => "Banking Supervision",
            Regime::DataProtection => "Data Protection",
            Regime::SanctionsEnforcement => "Sanctions Enforcement",
            Regime::ConsumerProtection => "Consumer Protection",
            Regime::Other(name) => name.as_str(),
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_corporate_prosecution() {
        assert_eq!(Regime::default(), Regime::CorporateProsecution);
    }

    #[test]
    fn parse_accepts_display_labels_and_variant_names() {
        assert_eq!(Regime::parse("Data Protection"), Regime::DataProtection);
        assert_eq!(Regime::parse("DataProtection"), Regime::DataProtection);
        assert_eq!(
            Regime::parse("banking supervision"),
            Regime::BankingSupervision
        );
        assert_eq!(
            Regime::parse("Sanctions Enforcement"),
            Regime::SanctionsEnforcement
        );
    }

    #[test]
    fn display_labels_reparse_to_the_same_variant() {
        // The UI regime dropdown is built from Display labels and fed back
        // through parse — every non-Other variant must survive that roundtrip.
        for regime in [
            Regime::CorporateProsecution,
            Regime::SecuritiesEnforcement,
            Regime::BankingSupervision,
            Regime::DataProtection,
            Regime::SanctionsEnforcement,
            Regime::ConsumerProtection,
        ] {
            assert_eq!(Regime::parse(&regime.to_string()), regime);
        }
    }

    #[test]
    fn parse_falls_back_to_other() {
        assert_eq!(
            Regime::parse("Telecom Licensing"),
            Regime::Other("Telecom Licensing".to_string())
        );
    }
}
