use serde::{Deserialize, Serialize};
use std::fmt;

/// Category of the underlying compliance failure that triggered the resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// FCPA / foreign bribery and corruption.
    Bribery,
    MoneyLaundering,
    SanctionsViolation,
    AntitrustFraud,
    SecuritiesFraud,
    TaxEvasion,
    ExportControl,
    /// GDPR/CCPA-style data-protection and privacy violations.
    DataProtection,
    /// Unfair/deceptive practices against consumers (FTC Act, UDAP).
    ConsumerProtection,
    Environmental,
    /// Insider trading and other market-abuse conduct.
    MarketAbuse,
    Other(String),
}

impl ViolationType {
    /// Maps free text (variant name, display label, or common shorthand —
    /// spacing/dashes/case ignored) onto a known variant, falling back to
    /// `Other` so nothing is dropped at a boundary.
    pub fn parse(value: &str) -> ViolationType {
        let normalized: String = value
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        match normalized.as_str() {
            "bribery" | "fcpa" => ViolationType::Bribery,
            "moneylaundering" => ViolationType::MoneyLaundering,
            "sanctionsviolation" => ViolationType::SanctionsViolation,
            "antitrustfraud" | "antitrust" => ViolationType::AntitrustFraud,
            "securitiesfraud" => ViolationType::SecuritiesFraud,
            "taxevasion" => ViolationType::TaxEvasion,
            "exportcontrol" => ViolationType::ExportControl,
            "dataprotection" | "gdpr" | "privacyviolation" => ViolationType::DataProtection,
            "consumerprotection" => ViolationType::ConsumerProtection,
            "environmental" => ViolationType::Environmental,
            "marketabuse" | "insidertrading" => ViolationType::MarketAbuse,
            _ => ViolationType::Other(value.trim().to_string()),
        }
    }
}

impl fmt::Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ViolationType::Bribery => "Bribery",
            ViolationType::MoneyLaundering => "Money Laundering",
            ViolationType::SanctionsViolation => "Sanctions Violation",
            ViolationType::AntitrustFraud => "Antitrust Fraud",
            ViolationType::SecuritiesFraud => "Securities Fraud",
            ViolationType::TaxEvasion => "Tax Evasion",
            ViolationType::ExportControl => "Export Control",
            ViolationType::DataProtection => "Data Protection",
            ViolationType::ConsumerProtection => "Consumer Protection",
            ViolationType::Environmental => "Environmental",
            ViolationType::MarketAbuse => "Market Abuse",
            ViolationType::Other(name) => name.as_str(),
        };
        f.write_str(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_display_labels_and_shorthands() {
        assert_eq!(
            ViolationType::parse("Data Protection"),
            ViolationType::DataProtection
        );
        assert_eq!(ViolationType::parse("GDPR"), ViolationType::DataProtection);
        assert_eq!(
            ViolationType::parse("Insider Trading"),
            ViolationType::MarketAbuse
        );
        assert_eq!(ViolationType::parse("fcpa"), ViolationType::Bribery);
    }

    #[test]
    fn parse_falls_back_to_other() {
        assert_eq!(
            ViolationType::parse("Jaywalking"),
            ViolationType::Other("Jaywalking".to_string())
        );
    }

    #[test]
    fn display_labels_reparse_to_the_same_variant() {
        // The UI dropdowns are built from Display labels and fed back through
        // parse — every non-Other variant must survive that roundtrip.
        for violation in [
            ViolationType::Bribery,
            ViolationType::MoneyLaundering,
            ViolationType::SanctionsViolation,
            ViolationType::AntitrustFraud,
            ViolationType::SecuritiesFraud,
            ViolationType::TaxEvasion,
            ViolationType::ExportControl,
            ViolationType::DataProtection,
            ViolationType::ConsumerProtection,
            ViolationType::Environmental,
            ViolationType::MarketAbuse,
        ] {
            assert_eq!(ViolationType::parse(&violation.to_string()), violation);
        }
    }
}
