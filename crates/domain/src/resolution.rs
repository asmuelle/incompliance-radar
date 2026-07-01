use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{Monitor, Regulator, Sanction, ViolationType};

/// The legal instrument used to resolve the enforcement action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionKind {
    DeferredProsecutionAgreement,
    NonProsecutionAgreement,
    ConsentOrder,
    /// Standalone independent compliance monitorship, not tied to a DPA/NPA.
    Monitorship,
    Other(String),
}

impl fmt::Display for ResolutionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ResolutionKind::DeferredProsecutionAgreement => "DPA",
            ResolutionKind::NonProsecutionAgreement => "NPA",
            ResolutionKind::ConsentOrder => "Consent Order",
            ResolutionKind::Monitorship => "Monitorship",
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

/// A single DPA/NPA/monitorship resolution extracted from a regulatory filing,
/// press release or court document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
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
