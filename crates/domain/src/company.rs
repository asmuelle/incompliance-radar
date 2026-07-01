use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompanyId(pub Uuid);

impl CompanyId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CompanyId {
    fn default() -> Self {
        Self::new()
    }
}

/// A company that is or was subject to a compliance monitorship, DPA or NPA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Company {
    pub id: CompanyId,
    pub name: String,
    pub industry: String,
    /// Primary jurisdiction of incorporation or listing, e.g. "US", "UK", "DE".
    pub jurisdiction: String,
    pub headquarters: Option<String>,
    pub ticker: Option<String>,
}

impl Company {
    pub fn new(
        name: impl Into<String>,
        industry: impl Into<String>,
        jurisdiction: impl Into<String>,
    ) -> Self {
        Self {
            id: CompanyId::new(),
            name: name.into(),
            industry: industry.into(),
            jurisdiction: jurisdiction.into(),
            headquarters: None,
            ticker: None,
        }
    }
}
