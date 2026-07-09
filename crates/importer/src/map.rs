//! Converts a loose [`RegistryRecord`] into `domain` types. This is the
//! validation boundary: everything the registry export leaves empty, zeroed
//! or free-form is normalized here, and nothing is invented — where the data
//! doesn't say, the domain field stays `None`/`Other(..)`.

use crate::record::RegistryRecord;
use chrono::NaiveDate;
use domain::{
    Monitor, Regulator, Resolution, ResolutionKind, ResolutionStatus, Sanction, ViolationType,
};

/// Upper bound assumed for a DPA/NPA term when the registry doesn't record
/// one, used only to decide Active vs Completed. The longest agreement in the
/// dataset is 120 months, but 60 covers all but a handful; erring long only
/// delays a case being shown as Completed, it never marks a live one done.
const ASSUMED_MAX_TERM_MONTHS: u32 = 60;

/// Obligation descriptions in the registry can run to paragraphs; keep the
/// stored obligation strings scannable.
const MAX_OBLIGATION_DESC_CHARS: usize = 240;

pub fn map_resolution(record: &RegistryRecord, today: NaiveDate) -> Resolution {
    let signed_on = NaiveDate::parse_from_str(record.date.trim(), "%Y-%m-%d").ok();
    let term_months =
        parse_months(&record.agreement_length).or(parse_months(&record.probation_length));

    Resolution {
        // The registry tracks federal organizational prosecutions, which are
        // DoJ actions by definition; parallel regulators (SEC, IRS, ...) are
        // listed in REG_AGENCY but the disposition itself is the DoJ's.
        regime: domain::Regime::CorporateProsecution,
        regulator: Regulator::doj(),
        kind: map_kind(&record.disposition_type),
        status: infer_status(signed_on, term_months, today),
        signed_on,
        term_months,
        monitor: parse_monitor(record),
        violations: map_violation(&record.primary_crime_code)
            .into_iter()
            .collect(),
        sanctions: build_sanctions(record),
        obligations: build_obligations(record),
        source: Some(source_citation(record)),
    }
}

pub fn map_kind(disposition: &str) -> ResolutionKind {
    match disposition.trim() {
        "DP" => ResolutionKind::DeferredProsecutionAgreement,
        "NP" => ResolutionKind::NonProsecutionAgreement,
        "plea" => ResolutionKind::Other("Plea Agreement".to_string()),
        "declination" => ResolutionKind::Other("Declination".to_string()),
        "trial" => ResolutionKind::Other("Trial Conviction".to_string()),
        "dismissal" => ResolutionKind::Other("Dismissal".to_string()),
        "acq" => ResolutionKind::Other("Acquittal".to_string()),
        other => ResolutionKind::Other(other.to_string()),
    }
}

/// Maps the registry's `PRIMARY_CRIME_CODE` labels onto `domain`'s known
/// variants where the concepts genuinely coincide; anything else keeps its
/// original registry label via `Other` rather than being force-fitted.
pub fn map_violation(code: &str) -> Option<ViolationType> {
    let code = code.trim();
    if code.is_empty() {
        return None;
    }
    Some(match code {
        "FCPA" | "Bribery" => ViolationType::Bribery,
        "Money Laundering" => ViolationType::MoneyLaundering,
        "Antitrust" => ViolationType::AntitrustFraud,
        "Fraud - Securities" => ViolationType::SecuritiesFraud,
        "Fraud - Tax" => ViolationType::TaxEvasion,
        "Import / Export" | "Import/Export" => ViolationType::ExportControl,
        other => ViolationType::Other(other.to_string()),
    })
}

/// Normalizes the registry's full country names to the short jurisdiction
/// codes the rest of the app uses ("US", "UK", ...; see `app::seed`) for the
/// countries that appear frequently in the dataset. Rare countries keep
/// their full name — an exact-match search filter on "Panama" is fine.
/// Some rows list several comma-separated countries (or the same one twice);
/// `Company.jurisdiction` is the *primary* jurisdiction, so only the first
/// is kept.
pub fn normalize_country(country: &str) -> String {
    let primary = country.split(',').next().unwrap_or(country);
    match primary.trim() {
        "" => "Unknown".to_string(),
        "United States of America" | "United States" => "US".to_string(),
        "United Kingdom" => "UK".to_string(),
        "Switzerland" => "CH".to_string(),
        "Japan" => "JP".to_string(),
        "Germany" => "DE".to_string(),
        "France" => "FR".to_string(),
        "Canada" => "CA".to_string(),
        "China" => "CN".to_string(),
        "South Korea" => "KR".to_string(),
        other => other.to_string(),
    }
}

/// Active vs Completed from what the registry records. A resolution is
/// Completed once its term has run out; with no term recorded we assume
/// [`ASSUMED_MAX_TERM_MONTHS`]. Undated rows are treated as historical
/// (Completed) — the registry reaches back to the 1990s and an undated row
/// carries no evidence of being live. The registry doesn't track breaches or
/// terminations, so those statuses never come from an import.
pub fn infer_status(
    signed_on: Option<NaiveDate>,
    term_months: Option<u32>,
    today: NaiveDate,
) -> ResolutionStatus {
    let Some(signed) = signed_on else {
        return ResolutionStatus::Completed;
    };
    let term = term_months.unwrap_or(ASSUMED_MAX_TERM_MONTHS);
    let end = signed.checked_add_months(chrono::Months::new(term));
    match end {
        Some(end) if end > today => ResolutionStatus::Active,
        _ => ResolutionStatus::Completed,
    }
}

/// The registry's `MONITOR_NAME` is free text, usually
/// `"Name; firm / role, background..."`, occasionally a note that the name
/// was withheld. Split off the name, keep the affiliation as `firm` so the
/// monitor-firm substring search matches it.
fn parse_monitor(record: &RegistryRecord) -> Option<Monitor> {
    if !is_true(&record.indep_monitor_req) {
        return None;
    }
    let raw = record.monitor_name.trim();
    let (name, firm) = if raw.is_empty() || raw.to_lowercase().starts_with("missing") {
        ("Independent monitor (name not public)".to_string(), None)
    } else {
        let mut parts = raw.splitn(2, ';');
        let name = parts.next().unwrap_or(raw).trim().to_string();
        let firm = parts
            .next()
            .map(str::trim)
            .filter(|f| !f.is_empty())
            .map(str::to_string);
        (name, firm)
    };
    Some(Monitor {
        name,
        firm,
        appointed_on: None,
        term_months: parse_months(&record.monitor_length)
            .or(parse_months(&record.agreement_length)),
    })
}

/// Prefers the itemized components (fine / forfeiture / restitution) and only
/// falls back to `TOTAL_PAYMENT` when no component is recorded — never both,
/// since the trend report sums sanction amounts per currency and including
/// the total alongside its components would double-count.
fn build_sanctions(record: &RegistryRecord) -> Vec<Sanction> {
    let components = [
        (&record.fine, "Criminal fine"),
        (&record.forfeiture_disgorgement, "Forfeiture / disgorgement"),
        (&record.restitution, "Restitution"),
    ];
    let sanctions: Vec<Sanction> = components
        .into_iter()
        .filter_map(|(raw, description)| {
            parse_amount(raw).map(|amount| Sanction {
                amount,
                currency: "USD".to_string(),
                description: Some(description.to_string()),
            })
        })
        .collect();
    if !sanctions.is_empty() {
        return sanctions;
    }
    parse_amount(&record.total_payment)
        .map(|amount| Sanction {
            amount,
            currency: "USD".to_string(),
            description: Some("Total payment".to_string()),
        })
        .into_iter()
        .collect()
}

fn build_obligations(record: &RegistryRecord) -> Vec<String> {
    let flags = [
        (
            &record.compliance_program_required,
            &record.compliance_program_desc,
            "Compliance program required by agreement",
        ),
        (
            &record.required_new_positions,
            &record.required_new_positions_desc,
            "New compliance positions required",
        ),
        (
            &record.required_outside_auditors,
            &record.required_outside_auditors_desc,
            "Outside auditors or experts required",
        ),
        (
            &record.compliance_officer_required,
            &record.compliance_officer_required_desc,
            "Compliance officer or consultant required",
        ),
        (
            &record.governance_changes_required,
            &record.governance_changes_required_desc,
            "Governance changes required",
        ),
    ];
    flags
        .into_iter()
        .filter(|(flag, _, _)| is_true(flag))
        .map(
            |(_, desc, label)| match truncate(desc.trim(), MAX_OBLIGATION_DESC_CHARS) {
                d if d.is_empty() => label.to_string(),
                d => format!("{label}: {d}"),
            },
        )
        .collect()
}

/// Citation recorded as the resolution's `source`. Not a URL — the registry
/// bulk export has no per-case permalinks — so it can never collide with the
/// crawler's URL-based dedup.
fn source_citation(record: &RegistryRecord) -> String {
    let case = match (record.case_name.trim(), record.case_id.trim()) {
        ("", "") => record.company.trim().to_string(),
        (name, "") => name.to_string(),
        ("", id) => id.to_string(),
        (name, id) => format!("{name}, {id}"),
    };
    match record.date.trim() {
        "" => format!("Corporate Prosecution Registry: {case}"),
        date => format!("Corporate Prosecution Registry: {case} ({date})"),
    }
}

fn is_true(raw: &str) -> bool {
    raw.trim().eq_ignore_ascii_case("true")
}

/// Positive amount from a plain numeric string; "0", empty and garbage all
/// mean "not recorded".
fn parse_amount(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok().filter(|v| *v > 0.0)
}

/// Positive month count; the registry writes "0" for unknown.
fn parse_months(raw: &str) -> Option<u32> {
    raw.trim().parse::<u32>().ok().filter(|v| *v > 0)
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let cut: String = text.chars().take(max_chars).collect();
    format!("{cut}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn maps_dp_and_np_to_dpa_and_npa() {
        assert_eq!(map_kind("DP"), ResolutionKind::DeferredProsecutionAgreement);
        assert_eq!(map_kind("NP"), ResolutionKind::NonProsecutionAgreement);
    }

    #[test]
    fn maps_other_dispositions_to_readable_labels() {
        assert_eq!(
            map_kind("plea"),
            ResolutionKind::Other("Plea Agreement".into())
        );
        assert_eq!(
            map_kind("declination"),
            ResolutionKind::Other("Declination".into())
        );
        assert_eq!(
            map_kind("trial"),
            ResolutionKind::Other("Trial Conviction".into())
        );
    }

    #[test]
    fn maps_known_crime_codes_to_domain_variants() {
        assert_eq!(map_violation("FCPA"), Some(ViolationType::Bribery));
        assert_eq!(
            map_violation("Money Laundering"),
            Some(ViolationType::MoneyLaundering)
        );
        assert_eq!(
            map_violation("Antitrust"),
            Some(ViolationType::AntitrustFraud)
        );
        assert_eq!(
            map_violation("Fraud - Securities"),
            Some(ViolationType::SecuritiesFraud)
        );
        assert_eq!(
            map_violation("Fraud - Tax"),
            Some(ViolationType::TaxEvasion)
        );
        assert_eq!(
            map_violation("Import / Export"),
            Some(ViolationType::ExportControl)
        );
    }

    #[test]
    fn keeps_unmapped_crime_codes_as_other_with_registry_label() {
        assert_eq!(
            map_violation("Environmental"),
            Some(ViolationType::Other("Environmental".into()))
        );
        assert_eq!(map_violation(""), None);
    }

    #[test]
    fn normalizes_frequent_countries_to_short_codes() {
        assert_eq!(normalize_country("United States of America"), "US");
        assert_eq!(normalize_country("United States"), "US");
        assert_eq!(normalize_country("United Kingdom"), "UK");
        assert_eq!(normalize_country("Panama"), "Panama");
        assert_eq!(normalize_country(""), "Unknown");
    }

    #[test]
    fn multi_country_rows_keep_only_the_primary_jurisdiction() {
        assert_eq!(
            normalize_country("United States of America,United States of America"),
            "US"
        );
        assert_eq!(
            normalize_country("Switzerland,United States of America"),
            "CH"
        );
    }

    #[test]
    fn status_is_active_while_term_is_running() {
        let today = date(2026, 7, 2);
        assert_eq!(
            infer_status(Some(date(2025, 1, 1)), Some(36), today),
            ResolutionStatus::Active
        );
    }

    #[test]
    fn status_is_completed_once_term_has_elapsed() {
        let today = date(2026, 7, 2);
        assert_eq!(
            infer_status(Some(date(2020, 1, 1)), Some(36), today),
            ResolutionStatus::Completed
        );
    }

    #[test]
    fn status_without_term_uses_assumed_maximum() {
        let today = date(2026, 7, 2);
        assert_eq!(
            infer_status(Some(date(2024, 1, 1)), None, today),
            ResolutionStatus::Active
        );
        assert_eq!(
            infer_status(Some(date(2019, 1, 1)), None, today),
            ResolutionStatus::Completed
        );
    }

    #[test]
    fn status_without_date_is_completed() {
        assert_eq!(
            infer_status(None, Some(36), date(2026, 7, 2)),
            ResolutionStatus::Completed
        );
    }

    #[test]
    fn monitor_name_and_firm_split_on_semicolon() {
        let record = RegistryRecord {
            indep_monitor_req: "True".into(),
            monitor_name: "Bart M. Schwartz; Chairman at Guidepost Solutions".into(),
            monitor_length: "36".into(),
            ..Default::default()
        };
        let monitor = parse_monitor(&record).unwrap();
        assert_eq!(monitor.name, "Bart M. Schwartz");
        assert_eq!(
            monitor.firm.as_deref(),
            Some("Chairman at Guidepost Solutions")
        );
        assert_eq!(monitor.term_months, Some(36));
    }

    #[test]
    fn withheld_monitor_name_gets_placeholder() {
        let record = RegistryRecord {
            indep_monitor_req: "True".into(),
            monitor_name: "Missing, declined to release name.".into(),
            ..Default::default()
        };
        let monitor = parse_monitor(&record).unwrap();
        assert_eq!(monitor.name, "Independent monitor (name not public)");
        assert_eq!(monitor.firm, None);
    }

    #[test]
    fn no_monitor_when_flag_is_false_or_empty() {
        let record = RegistryRecord {
            indep_monitor_req: "False".into(),
            monitor_name: "Someone".into(),
            ..Default::default()
        };
        assert!(parse_monitor(&record).is_none());
        assert!(parse_monitor(&RegistryRecord::default()).is_none());
    }

    #[test]
    fn sanctions_prefer_components_over_total_to_avoid_double_counting() {
        let record = RegistryRecord {
            total_payment: "300".into(),
            fine: "200".into(),
            restitution: "100".into(),
            ..Default::default()
        };
        let sanctions = build_sanctions(&record);
        assert_eq!(sanctions.len(), 2);
        let sum: f64 = sanctions.iter().map(|s| s.amount).sum();
        assert_eq!(sum, 300.0);
    }

    #[test]
    fn sanctions_fall_back_to_total_when_no_components_recorded() {
        let record = RegistryRecord {
            total_payment: "120400".into(),
            fine: "0".into(),
            ..Default::default()
        };
        let sanctions = build_sanctions(&record);
        assert_eq!(sanctions.len(), 1);
        assert_eq!(sanctions[0].amount, 120_400.0);
        assert_eq!(sanctions[0].description.as_deref(), Some("Total payment"));
    }

    #[test]
    fn zero_amounts_produce_no_sanctions() {
        let record = RegistryRecord {
            total_payment: "0".into(),
            ..Default::default()
        };
        assert!(build_sanctions(&record).is_empty());
    }

    #[test]
    fn obligations_come_from_true_flags_with_truncated_descriptions() {
        let record = RegistryRecord {
            compliance_program_required: "True".into(),
            compliance_program_desc: "Must adopt and maintain an effective program.".into(),
            governance_changes_required: "True".into(),
            required_new_positions: "False".into(),
            ..Default::default()
        };
        let obligations = build_obligations(&record);
        assert_eq!(obligations.len(), 2);
        assert!(obligations[0].starts_with("Compliance program required by agreement: Must adopt"));
        assert_eq!(obligations[1], "Governance changes required");
    }

    #[test]
    fn resolution_maps_end_to_end_from_a_realistic_record() {
        let record = RegistryRecord {
            company: "Example Corp".into(),
            disposition_type: "DP".into(),
            primary_crime_code: "FCPA".into(),
            date: "2024-03-15".into(),
            agreement_length: "36".into(),
            fine: "200000000".into(),
            case_name: "USA v. Example Corp".into(),
            case_id: "1:24-cr-00001".into(),
            ..Default::default()
        };
        let resolution = map_resolution(&record, date(2026, 7, 2));

        assert_eq!(resolution.regulator, Regulator::doj());
        assert_eq!(
            resolution.kind,
            ResolutionKind::DeferredProsecutionAgreement
        );
        assert_eq!(resolution.status, ResolutionStatus::Active);
        assert_eq!(resolution.signed_on, Some(date(2024, 3, 15)));
        assert_eq!(resolution.term_months, Some(36));
        assert_eq!(resolution.violations, vec![ViolationType::Bribery]);
        assert_eq!(resolution.sanctions.len(), 1);
        assert_eq!(
            resolution.source.as_deref(),
            Some("Corporate Prosecution Registry: USA v. Example Corp, 1:24-cr-00001 (2024-03-15)")
        );
    }
}
