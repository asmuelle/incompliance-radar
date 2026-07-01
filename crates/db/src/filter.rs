/// Criteria for [`crate::CaseRepository::search`]. All fields are optional
/// and combined with AND — a filter with every field `None` matches every
/// case, same as [`crate::CaseRepository::list`].
///
/// `industry`/`jurisdiction` match exactly (case-insensitive) since they're
/// indexed SQL columns; `violation_type`/`monitor_firm` match against a
/// case's resolutions (case-insensitive substring for `monitor_firm`, since
/// firm names are free text) — see `SqliteCaseRepository::search` for why
/// that part isn't push down into SQL.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaseFilter {
    pub industry: Option<String>,
    pub jurisdiction: Option<String>,
    pub violation_type: Option<String>,
    pub monitor_firm: Option<String>,
}

impl CaseFilter {
    pub fn is_empty(&self) -> bool {
        self.industry.is_none()
            && self.jurisdiction.is_none()
            && self.violation_type.is_none()
            && self.monitor_firm.is_none()
    }
}
