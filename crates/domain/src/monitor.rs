use serde::{Deserialize, Serialize};

/// An independent compliance monitor appointed to oversee a company's remediation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Monitor {
    pub name: String,
    /// Law firm or consultancy the monitor is affiliated with, if publicly disclosed.
    pub firm: Option<String>,
    pub appointed_on: Option<chrono::NaiveDate>,
    pub term_months: Option<u32>,
}
