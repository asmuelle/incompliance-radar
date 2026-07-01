use crate::ExtractionError;
use domain::{
    Company, ComplianceCase, Monitor, Regulator, Resolution, ResolutionKind, ResolutionStatus,
    Sanction, ViolationType,
};
use serde::Deserialize;

/// Mirrors the JSON shape described in `prompt::SYSTEM_PROMPT`. Deliberately
/// looser than the `domain` types it converts into — fields are plain
/// strings here, and `try_into_domain` is where untrusted model output gets
/// validated against `domain`'s actual invariants (see rules/common/security.md:
/// "parse, don't validate" at the system boundary).
#[derive(Debug, Deserialize)]
pub(crate) struct ParsedCase {
    company_name: String,
    industry: String,
    jurisdiction: String,
    regulator: String,
    resolution_kind: String,
    status: String,
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

impl ParsedCase {
    pub(crate) fn try_into_domain(self) -> Result<ComplianceCase, ExtractionError> {
        if self.company_name.trim().is_empty() {
            return Err(ExtractionError::Validation(
                "company_name must not be empty".into(),
            ));
        }

        let company = Company::new(self.company_name, self.industry, self.jurisdiction);
        let mut case = ComplianceCase::new(company);

        case.resolutions.push(Resolution {
            regulator: parse_regulator(&self.regulator),
            kind: parse_resolution_kind(&self.resolution_kind),
            status: parse_status(&self.status)?,
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

        Ok(case)
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
        let case = parsed.try_into_domain().unwrap();

        assert_eq!(case.company.name, "Acme Global Industries");
        assert_eq!(case.resolutions.len(), 1);
        let resolution = &case.resolutions[0];
        assert_eq!(resolution.regulator, Regulator::Doj);
        assert_eq!(resolution.status, ResolutionStatus::Active);
        assert_eq!(resolution.violations, vec![ViolationType::Bribery]);
        assert_eq!(resolution.monitor.as_ref().unwrap().name, "Jane Doe");
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

        let case = parsed.try_into_domain().unwrap();

        let resolution = &case.resolutions[0];
        assert!(resolution.violations.is_empty());
        assert!(resolution.sanctions.is_empty());
        assert!(resolution.obligations.is_empty());
        assert!(resolution.monitor.is_none());
    }
}
