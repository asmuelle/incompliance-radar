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
    Other(String),
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
            ViolationType::Other(name) => name.as_str(),
        };
        f.write_str(label)
    }
}
