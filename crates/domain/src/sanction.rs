use serde::{Deserialize, Serialize};

/// A financial penalty imposed as part of a resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sanction {
    pub amount: f64,
    pub currency: String,
    pub description: Option<String>,
}
