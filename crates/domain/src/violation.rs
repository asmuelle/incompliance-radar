use serde::{Deserialize, Serialize};

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
