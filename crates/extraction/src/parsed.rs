use crate::ExtractionError;
use domain::{
    Company, ComplianceCase, Monitor, Regulator, Resolution, ResolutionKind, ResolutionStatus,
    Sanction, ViolationType,
};
use serde::Deserialize;

/// Mirrors the JSON shape described in `prompt::SYSTEM_PROMPT`. Deliberately
/// looser than the `domain` types it converts into — every field is
/// `Option`, even the two (`company_name`, `status`) the prompt asks the
/// model never to leave null, because live testing showed it sometimes does
/// anyway. `try_into_domain` is where untrusted model output gets validated
/// against `domain`'s actual invariants (see rules/common/security.md: "parse,
/// don't validate" at the system boundary) — a null we can default
/// sensibly (industry, jurisdiction, regulator, resolution_kind) becomes a
/// default; a null we can't (company_name, status) becomes a clear
/// `ExtractionError::Validation`, not a generic `serde` deserialize crash.
#[derive(Debug, Deserialize)]
pub(crate) struct ParsedCase {
    company_name: Option<String>,
    industry: Option<String>,
    jurisdiction: Option<String>,
    regulator: Option<String>,
    resolution_kind: Option<String>,
    status: Option<String>,
    signed_on: Option<String>,
    term_months: Option<u32>,
    monitor: Option<ParsedMonitor>,
    #[serde(default)]
    violations: Vec<String>,
    #[serde(default)]
    sanctions: Vec<ParsedSanction>,
    #[serde(default)]
    obligations: Vec<String>,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ParsedMonitor {
    name: String,
    firm: Option<String>,
    appointed_on: Option<String>,
    term_months: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ParsedSanction {
    amount: f64,
    currency: String,
    description: Option<String>,
}

/// Used when the model doesn't identify a secondary field (industry,
/// jurisdiction, regulator, resolution kind) — unlike `company_name` and
/// `status`, none of these are essential to what makes this a tracked case,
/// so a placeholder beats rejecting the whole extraction over it.
const UNKNOWN: &str = "Unknown";

/// Seen live: instead of returning the standalone `{"not_applicable": true}`
/// sentinel, the model sometimes emits a full case-shaped object but writes
/// the literal field name back as `company_name`'s value. Any of these,
/// trimmed and lowercased, is treated the same as the proper sentinel.
const NOT_APPLICABLE_MARKERS: &[&str] = &["not_applicable", "not applicable", "n/a"];

fn looks_like_not_applicable_marker(value: &str) -> bool {
    NOT_APPLICABLE_MARKERS.contains(&value.trim().to_lowercase().as_str())
}

impl ParsedCase {
    /// `Ok(None)` covers both the proper `not_applicable` sentinel (handled
    /// one level up, before this is even called) and this same intent
    /// expressed the "wrong" way, inside an otherwise full object — see
    /// `looks_like_not_applicable_marker`.
    pub(crate) fn try_into_domain(self) -> Result<Option<ComplianceCase>, ExtractionError> {
        let company_name = self
            .company_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                ExtractionError::Validation(
                    "company_name is required but was missing or empty".into(),
                )
            })?;

        if looks_like_not_applicable_marker(company_name) {
            return Ok(None);
        }
        let company_name = company_name.to_string();

        let status = self
            .status
            .as_deref()
            .ok_or_else(|| ExtractionError::Validation("status is required but was missing".into()))
            .and_then(parse_status)?;

        let company = Company::new(
            company_name,
            self.industry.unwrap_or_else(|| UNKNOWN.to_string()),
            self.jurisdiction.unwrap_or_else(|| UNKNOWN.to_string()),
        );
        let mut case = ComplianceCase::new(company);

        case.resolutions.push(Resolution {
            regulator: self
                .regulator
                .as_deref()
                .map(parse_regulator)
                .unwrap_or(Regulator::Other(UNKNOWN.to_string())),
            kind: self
                .resolution_kind
                .as_deref()
                .map(parse_resolution_kind)
                .unwrap_or(ResolutionKind::Other(UNKNOWN.to_string())),
            status,
            signed_on: parse_date(self.signed_on.as_deref())?,
            term_months: self.term_months,
            monitor: self
                .monitor
                .map(ParsedMonitor::try_into_domain)
                .transpose()?,
            violations: self.violations.iter().map(|v| parse_violation(v)).collect(),
            sanctions: self
                .sanctions
                .into_iter()
                .map(ParsedSanction::try_into_domain)
                .collect::<Result<_, _>>()?,
            obligations: self.obligations,
            source: self.source,
        });

        Ok(Some(case))
    }
}

impl ParsedMonitor {
    fn try_into_domain(self) -> Result<Monitor, ExtractionError> {
        Ok(Monitor {
            name: self.name,
            firm: self.firm,
            appointed_on: parse_date(self.appointed_on.as_deref())?,
            term_months: self.term_months,
        })
    }
}

impl ParsedSanction {
    fn try_into_domain(self) -> Result<Sanction, ExtractionError> {
        if self.amount < 0.0 {
            return Err(ExtractionError::Validation(format!(
                "sanction amount must not be negative, got {}",
                self.amount
            )));
        }
        if self.currency.trim().is_empty() {
            return Err(ExtractionError::Validation(
                "sanction currency must not be empty".into(),
            ));
        }
        Ok(Sanction {
            amount: self.amount,
            currency: self.currency,
            description: self.description,
        })
    }
}

fn parse_date(value: Option<&str>) -> Result<Option<chrono::NaiveDate>, ExtractionError> {
    value
        .map(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
                ExtractionError::Validation(format!("invalid date '{s}', expected YYYY-MM-DD"))
            })
        })
        .transpose()
}

fn parse_status(value: &str) -> Result<ResolutionStatus, ExtractionError> {
    match value.to_lowercase().as_str() {
        "active" => Ok(ResolutionStatus::Active),
        "completed" => Ok(ResolutionStatus::Completed),
        "terminated" => Ok(ResolutionStatus::Terminated),
        "breached" => Ok(ResolutionStatus::Breached),
        other => Err(ExtractionError::Validation(format!(
            "unknown status '{other}', expected one of active/completed/terminated/breached"
        ))),
    }
}

fn parse_regulator(value: &str) -> Regulator {
    match value.to_lowercase().as_str() {
        "doj" => Regulator::Doj,
        "sec" => Regulator::Sec,
        "fca" => Regulator::Fca,
        "ofac" => Regulator::Ofac,
        "sfo" => Regulator::Sfo,
        _ => Regulator::Other(value.to_string()),
    }
}

fn normalize(value: &str) -> String {
    value.to_lowercase().replace([' ', '-', '_'], "")
}

fn parse_resolution_kind(value: &str) -> ResolutionKind {
    match normalize(value).as_str() {
        "deferredprosecutionagreement" | "dpa" => ResolutionKind::DeferredProsecutionAgreement,
        "nonprosecutionagreement" | "npa" => ResolutionKind::NonProsecutionAgreement,
        "consentorder" => ResolutionKind::ConsentOrder,
        "monitorship" => ResolutionKind::Monitorship,
        _ => ResolutionKind::Other(value.to_string()),
    }
}

fn parse_violation(value: &str) -> ViolationType {
    match normalize(value).as_str() {
        "bribery" | "fcpa" => ViolationType::Bribery,
        "moneylaundering" => ViolationType::MoneyLaundering,
        "sanctionsviolation" => ViolationType::SanctionsViolation,
        "antitrustfraud" | "antitrust" => ViolationType::AntitrustFraud,
        "securitiesfraud" => ViolationType::SecuritiesFraud,
        "taxevasion" => ViolationType::TaxEvasion,
        "exportcontrol" => ViolationType::ExportControl,
        _ => ViolationType::Other(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> &'static str {
        r#"{
            "company_name": "Acme Global Industries",
            "industry": "Manufacturing",
            "jurisdiction": "US",
            "regulator": "Doj",
            "resolution_kind": "DeferredProsecutionAgreement",
            "status": "Active",
            "signed_on": "2024-03-15",
            "term_months": 36,
            "monitor": { "name": "Jane Doe", "firm": "Example LLP", "appointed_on": "2024-04-01", "term_months": 36 },
            "violations": ["Bribery"],
            "sanctions": [{ "amount": 45000000.0, "currency": "USD", "description": "Criminal penalty" }],
            "obligations": ["Report quarterly"],
            "source": "https://example.com/filing"
        }"#
    }

    #[test]
    fn valid_json_converts_to_domain_case() {
        let parsed: ParsedCase = serde_json::from_str(valid_json()).unwrap();
        let case = parsed
            .try_into_domain()
            .unwrap()
            .expect("should be a real case");

        assert_eq!(case.company.name, "Acme Global Industries");
        assert_eq!(case.resolutions.len(), 1);
        let resolution = &case.resolutions[0];
        assert_eq!(resolution.regulator, Regulator::Doj);
        assert_eq!(resolution.status, ResolutionStatus::Active);
        assert_eq!(resolution.violations, vec![ViolationType::Bribery]);
        assert_eq!(resolution.monitor.as_ref().unwrap().name, "Jane Doe");
    }

    #[test]
    fn company_name_literally_not_applicable_is_treated_as_not_a_case() {
        // Regression test: seen live against a real FCA speech transcript —
        // the model returned a full case-shaped object (not the standalone
        // `{"not_applicable": true}` sentinel) but wrote the sentinel word
        // into `company_name` instead of a real name.
        let json = valid_json().replace("\"Acme Global Industries\"", "\"not_applicable\"");
        let parsed: ParsedCase = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.try_into_domain().unwrap(), None);
    }

    #[test]
    fn empty_company_name_is_rejected() {
        let json = valid_json().replace("\"Acme Global Industries\"", "\"  \"");
        let parsed: ParsedCase = serde_json::from_str(&json).unwrap();

        let err = parsed.try_into_domain().unwrap_err();

        assert!(matches!(err, ExtractionError::Validation(_)));
    }

    #[test]
    fn unknown_status_is_rejected() {
        let json = valid_json().replace("\"Active\"", "\"Ongoing\"");
        let parsed: ParsedCase = serde_json::from_str(&json).unwrap();

        let err = parsed.try_into_domain().unwrap_err();

        assert!(matches!(err, ExtractionError::Validation(_)));
    }

    #[test]
    fn negative_sanction_amount_is_rejected() {
        let json = valid_json().replace("45000000.0", "-1.0");
        let parsed: ParsedCase = serde_json::from_str(&json).unwrap();

        let err = parsed.try_into_domain().unwrap_err();

        assert!(matches!(err, ExtractionError::Validation(_)));
    }

    #[test]
    fn invalid_date_is_rejected() {
        let json = valid_json().replace("2024-03-15", "15/03/2024");
        let parsed: ParsedCase = serde_json::from_str(&json).unwrap();

        let err = parsed.try_into_domain().unwrap_err();

        assert!(matches!(err, ExtractionError::Validation(_)));
    }

    #[test]
    fn unrecognized_regulator_falls_back_to_other() {
        assert_eq!(
            parse_regulator("BaFin"),
            Regulator::Other("BaFin".to_string())
        );
    }

    #[test]
    fn unrecognized_violation_falls_back_to_other() {
        assert_eq!(
            parse_violation("Insider Trading"),
            ViolationType::Other("Insider Trading".to_string())
        );
    }

    #[test]
    fn null_company_name_is_rejected() {
        let json = valid_json().replace("\"Acme Global Industries\"", "null");
        let parsed: ParsedCase = serde_json::from_str(&json).unwrap();

        let err = parsed.try_into_domain().unwrap_err();

        assert!(matches!(err, ExtractionError::Validation(_)));
    }

    #[test]
    fn null_status_is_rejected() {
        let json = valid_json().replace("\"Active\"", "null");
        let parsed: ParsedCase = serde_json::from_str(&json).unwrap();

        let err = parsed.try_into_domain().unwrap_err();

        assert!(matches!(err, ExtractionError::Validation(_)));
    }

    #[test]
    fn null_secondary_fields_fall_back_to_defaults_instead_of_failing() {
        // Regression test: a real FCA press release made the model return
        // `company_name`/`status` correctly but `null` for other "required"
        // fields, which used to crash `serde_json::from_str` outright.
        let json = r#"{
            "company_name": "Acme",
            "industry": null,
            "jurisdiction": null,
            "regulator": null,
            "resolution_kind": null,
            "status": "Active",
            "signed_on": null,
            "term_months": null,
            "monitor": null,
            "source": null
        }"#;
        let parsed: ParsedCase = serde_json::from_str(json).unwrap();

        let case = parsed
            .try_into_domain()
            .unwrap()
            .expect("should be a real case");

        assert_eq!(case.company.industry, UNKNOWN);
        assert_eq!(case.company.jurisdiction, UNKNOWN);
        assert_eq!(
            case.resolutions[0].regulator,
            Regulator::Other(UNKNOWN.to_string())
        );
        assert_eq!(
            case.resolutions[0].kind,
            ResolutionKind::Other(UNKNOWN.to_string())
        );
    }

    #[test]
    fn missing_optional_fields_default_to_empty() {
        let json = r#"{
            "company_name": "Acme",
            "industry": "Manufacturing",
            "jurisdiction": "US",
            "regulator": "Sec",
            "resolution_kind": "ConsentOrder",
            "status": "Completed",
            "signed_on": null,
            "term_months": null,
            "monitor": null,
            "source": null
        }"#;
        let parsed: ParsedCase = serde_json::from_str(json).unwrap();

        let case = parsed
            .try_into_domain()
            .unwrap()
            .expect("should be a real case");

        let resolution = &case.resolutions[0];
        assert!(resolution.violations.is_empty());
        assert!(resolution.sanctions.is_empty());
        assert!(resolution.obligations.is_empty());
        assert!(resolution.monitor.is_none());
    }
}
