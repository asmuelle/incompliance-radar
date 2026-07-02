//! Loose CSV-row DTO for the Corporate Prosecution Registry bulk download
//! (`corp-crime.csv`). Deliberately all plain strings — the registry export
//! leaves most fields empty and mixes formats, so validation and conversion
//! into `domain` types happens in `map.rs`, mirroring how
//! `extraction::parsed::ParsedCase` keeps the LLM boundary loose.
//!
//! Only the columns the importer actually maps are listed; `csv` +
//! `serde` ignore the rest of the ~70 columns in the export.

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RegistryRecord {
    #[serde(rename = "COMPANY", default)]
    pub company: String,
    #[serde(rename = "DISPOSITION_TYPE", default)]
    pub disposition_type: String,
    #[serde(rename = "PRIMARY_CRIME_CODE", default)]
    pub primary_crime_code: String,
    #[serde(rename = "COUNTRY", default)]
    pub country: String,
    #[serde(rename = "NAICS", default)]
    pub naics: String,
    #[serde(rename = "TICKER", default)]
    pub ticker: String,
    /// Agreement/disposition date, `YYYY-MM-DD` when present.
    #[serde(rename = "DATE", default)]
    pub date: String,
    #[serde(rename = "CASE_NAME", default)]
    pub case_name: String,
    /// Court docket number, e.g. `1:14-cr-00027`. Not unique across districts.
    #[serde(rename = "CASE_ID", default)]
    pub case_id: String,
    /// All payment fields are plain integer/decimal USD amounts (no symbols).
    #[serde(rename = "TOTAL_PAYMENT", default)]
    pub total_payment: String,
    #[serde(rename = "FINE", default)]
    pub fine: String,
    #[serde(rename = "FORFEITURE_DISGORGEMENT", default)]
    pub forfeiture_disgorgement: String,
    #[serde(rename = "RESTITUTION", default)]
    pub restitution: String,
    /// Lengths are in months ("36", "24", ...); "0" or empty means unknown.
    #[serde(rename = "AGREEMENT_LENGTH", default)]
    pub agreement_length: String,
    #[serde(rename = "PROBATION_LENGTH", default)]
    pub probation_length: String,
    /// "True" / "False" / empty.
    #[serde(rename = "INDEP_MONITOR_REQ", default)]
    pub indep_monitor_req: String,
    /// Free text, usually "Name; firm/role, extra background...". Sometimes
    /// literally "Missing, declined to release name.".
    #[serde(rename = "MONITOR_NAME", default)]
    pub monitor_name: String,
    #[serde(rename = "MONITOR_LENGTH", default)]
    pub monitor_length: String,
    #[serde(rename = "COMPLIANCE_PROGRAM_REQUIRED_BY_AGREEMENT", default)]
    pub compliance_program_required: String,
    #[serde(rename = "COMPLIANCE_PROGRAM_DESC", default)]
    pub compliance_program_desc: String,
    #[serde(rename = "AGREEMENT_REQUIRED_NEW_POSITIONS", default)]
    pub required_new_positions: String,
    #[serde(rename = "AGREEMENT_REQUIRED_NEW_POSITIONS_DESC", default)]
    pub required_new_positions_desc: String,
    #[serde(rename = "AGREEMENT_REQUIRED_OUTSIDE_AUDITORS_OR_EXPERTS", default)]
    pub required_outside_auditors: String,
    #[serde(
        rename = "AGREEMENT_REQUIRED_OUTSIDE_AUDITORS_OR_EXPERTS_DESC",
        default
    )]
    pub required_outside_auditors_desc: String,
    #[serde(rename = "OTHER_COMPLIANCE_OFFICER_OR_CONSULTANT_REQUIRED", default)]
    pub compliance_officer_required: String,
    #[serde(
        rename = "OTHER_COMPLIANCE_OFFICER_OR_CONSULTANT_REQUIRED_DESC",
        default
    )]
    pub compliance_officer_required_desc: String,
    #[serde(rename = "OTHER_AGREEMENT_REQUIRED_GOVERNANCE_CHANGES", default)]
    pub governance_changes_required: String,
    #[serde(rename = "OTHER_AGREEMENT_REQUIRED_GOVERNANCE_CHANGES_DESC", default)]
    pub governance_changes_required_desc: String,
}
