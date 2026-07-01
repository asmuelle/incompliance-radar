use serde::{Deserialize, Serialize};
use std::fmt;

/// Regulatory or enforcement body that brought or oversees the case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Regulator {
    /// US Department of Justice
    Doj,
    /// US Securities and Exchange Commission
    Sec,
    /// UK Financial Conduct Authority
    Fca,
    /// US Office of Foreign Assets Control
    Ofac,
    /// UK Serious Fraud Office
    Sfo,
    Other(String),
}

impl fmt::Display for Regulator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Regulator::Doj => "DoJ",
            Regulator::Sec => "SEC",
            Regulator::Fca => "FCA",
            Regulator::Ofac => "OFAC",
            Regulator::Sfo => "SFO",
            Regulator::Other(name) => name.as_str(),
        };
        f.write_str(label)
    }
}
