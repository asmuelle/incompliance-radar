//! Bulk import of the Corporate Prosecution Registry dataset
//! (<https://corporate-prosecution-registry.com/downloads/>) into the case
//! repository — the historical backfill that turns the app's demo dataset
//! into a real precedent corpus. Run via the `import-registry` binary
//! (`src/bin/import_registry.rs`); see CLAUDE.md's Importer section for
//! usage, default disposition filter and the data-licensing caveat.
//!
//! Re-running an import is safe: case IDs are deterministic (UUIDv5 of the
//! normalized company name), so a re-import updates existing rows instead of
//! duplicating them. Cases created by the crawler or manual extraction use
//! random IDs and are never touched.
//!
//! Unlike the crawler, a bulk import deliberately does **not** evaluate
//! watch rules — flooding the alert feed with hundreds of historical matches
//! would bury any real, current alert.

mod map;
mod naics;
mod record;

pub use record::RegistryRecord;

use chrono::NaiveDate;
use domain::{Company, CompanyId, ComplianceCase};
use std::collections::BTreeMap;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ImporterError {
    #[error("failed to read registry CSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("repository error: {0}")]
    Repository(#[from] db::RepositoryError),
}

/// Dispositions imported by default: the on-spec DPA/NPA corpus plus DoJ
/// declinations. The registry's ~3,600 plea agreements and other outcomes
/// are one `--dispositions` flag away, but swamping the (unpaginated) UI
/// with every environmental plea since the 1990s isn't the default.
pub const DEFAULT_DISPOSITIONS: &[&str] = &["DP", "NP", "declination"];

#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// Raw `DISPOSITION_TYPE` values to import, compared case-insensitively.
    pub dispositions: Vec<String>,
    /// Parse and map without touching the repository.
    pub dry_run: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            dispositions: DEFAULT_DISPOSITIONS.iter().map(|d| d.to_string()).collect(),
            dry_run: false,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImportSummary {
    pub rows_total: usize,
    pub rows_selected: usize,
    pub rows_skipped_disposition: usize,
    pub rows_skipped_missing_company: usize,
    pub cases_built: usize,
    pub cases_upserted: usize,
    pub upsert_failures: usize,
}

/// Reads, maps and persists the registry CSV at `path`. One case failing to
/// persist is logged and counted, not fatal — same policy as the crawler.
pub async fn import_registry_csv(
    path: &Path,
    options: &ImportOptions,
    repo: &dyn db::CaseRepository,
    today: NaiveDate,
) -> Result<ImportSummary, ImporterError> {
    let records = read_records(path)?;
    let (cases, mut summary) = build_cases(&records, options, today);

    if options.dry_run {
        tracing::info!(?summary, "dry run — skipping persistence");
        return Ok(summary);
    }

    for case in &cases {
        match repo.upsert(case).await {
            Ok(()) => summary.cases_upserted += 1,
            Err(err) => {
                tracing::error!(company = %case.company.name, %err, "failed to persist imported case");
                summary.upsert_failures += 1;
            }
        }
    }
    Ok(summary)
}

pub fn read_records(path: &Path) -> Result<Vec<RegistryRecord>, ImporterError> {
    let mut reader = csv::Reader::from_path(path)?;
    let records = reader.deserialize().collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

/// Groups the selected rows by normalized company name — the registry has no
/// stable company key and docket numbers repeat across districts — building
/// one [`ComplianceCase`] per company with all its resolutions, oldest first.
pub fn build_cases(
    records: &[RegistryRecord],
    options: &ImportOptions,
    today: NaiveDate,
) -> (Vec<ComplianceCase>, ImportSummary) {
    let mut summary = ImportSummary {
        rows_total: records.len(),
        ..Default::default()
    };

    let mut by_company: BTreeMap<String, Vec<&RegistryRecord>> = BTreeMap::new();
    for record in records {
        if record.company.trim().is_empty() {
            summary.rows_skipped_missing_company += 1;
            continue;
        }
        if !disposition_selected(&record.disposition_type, &options.dispositions) {
            summary.rows_skipped_disposition += 1;
            continue;
        }
        summary.rows_selected += 1;
        by_company
            .entry(company_key(&record.company))
            .or_default()
            .push(record);
    }

    let cases: Vec<ComplianceCase> = by_company
        .into_iter()
        .map(|(key, rows)| build_case(&key, &rows, today))
        .collect();
    summary.cases_built = cases.len();
    (cases, summary)
}

fn build_case(key: &str, rows: &[&RegistryRecord], today: NaiveDate) -> ComplianceCase {
    // Oldest first so both company-field precedence (below) and the stored
    // resolution history read chronologically; undated rows sort first.
    let mut rows = rows.to_vec();
    rows.sort_by_key(|r| r.date.trim().to_string());

    // Latest non-empty value wins for company fields — the most recent row
    // reflects the registry's freshest picture of the company.
    let mut industry: Option<&str> = None;
    let mut country: Option<&str> = None;
    let mut ticker: Option<&str> = None;
    for row in &rows {
        if let Some(sector) = naics::sector_name(&row.naics) {
            industry = Some(sector);
        }
        if !row.country.trim().is_empty() {
            country = Some(row.country.trim());
        }
        // Multi-defendant rows comma-join tickers (",HSBC"); the first
        // non-empty segment is the lead defendant's.
        if let Some(t) = row.ticker.split(',').map(str::trim).find(|t| !t.is_empty()) {
            ticker = Some(t);
        }
    }

    let display_name = rows
        .last()
        .map(|r| r.company.trim().to_string())
        .unwrap_or_else(|| key.to_string());

    let company = Company {
        id: CompanyId(deterministic_id(&format!("company:{key}"))),
        name: display_name,
        industry: industry.unwrap_or("Unknown").to_string(),
        jurisdiction: map::normalize_country(country.unwrap_or("")),
        headquarters: None,
        ticker: ticker.map(str::to_string),
    };

    ComplianceCase {
        id: deterministic_id(&format!("case:{key}")),
        company,
        resolutions: rows.iter().map(|r| map::map_resolution(r, today)).collect(),
    }
}

fn disposition_selected(disposition: &str, selected: &[String]) -> bool {
    selected
        .iter()
        .any(|s| s.trim().eq_ignore_ascii_case(disposition.trim()))
}

fn company_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// UUIDv5 under a fixed namespace so the same registry company always maps
/// to the same case ID across imports.
fn deterministic_id(key: &str) -> Uuid {
    let namespace = Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"registry-import.incompliance-radar");
    Uuid::new_v5(&namespace, key.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{ResolutionKind, ResolutionStatus};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 2).unwrap()
    }

    fn dp_record(company: &str, date: &str) -> RegistryRecord {
        RegistryRecord {
            company: company.to_string(),
            disposition_type: "DP".to_string(),
            date: date.to_string(),
            country: "United States of America".to_string(),
            naics: "522110".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn groups_rows_for_the_same_company_into_one_case() {
        let records = vec![
            dp_record("Acme Bank", "2020-01-01"),
            dp_record("ACME BANK ", "2024-06-01"),
            dp_record("Other Corp", "2023-01-01"),
        ];

        let (cases, summary) = build_cases(&records, &ImportOptions::default(), today());

        assert_eq!(summary.rows_selected, 3);
        assert_eq!(cases.len(), 2);
        let acme = cases
            .iter()
            .find(|c| c.company.name == "ACME BANK")
            .unwrap();
        assert_eq!(acme.resolutions.len(), 2);
        assert_eq!(
            acme.resolutions[0].signed_on,
            NaiveDate::from_ymd_opt(2020, 1, 1)
        );
    }

    #[test]
    fn default_options_skip_pleas_and_dismissals() {
        let mut plea = dp_record("Plea Co", "2020-01-01");
        plea.disposition_type = "plea".to_string();
        let mut dismissal = dp_record("Dismissed Co", "2020-01-01");
        dismissal.disposition_type = "dismissal".to_string();
        let records = vec![dp_record("DPA Co", "2020-01-01"), plea, dismissal];

        let (cases, summary) = build_cases(&records, &ImportOptions::default(), today());

        assert_eq!(cases.len(), 1);
        assert_eq!(summary.rows_selected, 1);
        assert_eq!(summary.rows_skipped_disposition, 2);
    }

    #[test]
    fn widened_dispositions_include_pleas() {
        let mut plea = dp_record("Plea Co", "2020-01-01");
        plea.disposition_type = "plea".to_string();
        let options = ImportOptions {
            dispositions: vec!["DP".into(), "plea".into()],
            ..Default::default()
        };

        let (cases, _) = build_cases(&[plea], &options, today());

        assert_eq!(cases.len(), 1);
        assert_eq!(
            cases[0].resolutions[0].kind,
            ResolutionKind::Other("Plea Agreement".into())
        );
    }

    #[test]
    fn rows_without_company_are_skipped_and_counted() {
        let records = vec![dp_record("", "2020-01-01")];
        let (cases, summary) = build_cases(&records, &ImportOptions::default(), today());
        assert!(cases.is_empty());
        assert_eq!(summary.rows_skipped_missing_company, 1);
    }

    #[test]
    fn case_ids_are_deterministic_across_builds() {
        let records = vec![dp_record("Acme Bank", "2020-01-01")];
        let (first, _) = build_cases(&records, &ImportOptions::default(), today());
        let (second, _) = build_cases(&records, &ImportOptions::default(), today());
        assert_eq!(first[0].id, second[0].id);
        assert_eq!(first[0].company.id, second[0].company.id);
    }

    #[test]
    fn ticker_takes_first_non_empty_comma_segment() {
        let mut record = dp_record("HSBC Bank USA, N.A.,HSBC Holdings Plc", "2012-12-11");
        record.ticker = ",HSBC".to_string();
        let (cases, _) = build_cases(&[record], &ImportOptions::default(), today());
        assert_eq!(cases[0].company.ticker.as_deref(), Some("HSBC"));
    }

    #[test]
    fn company_fields_map_from_registry_row() {
        let records = vec![dp_record("Acme Bank", "2025-06-01")];
        let (cases, _) = build_cases(&records, &ImportOptions::default(), today());
        let company = &cases[0].company;
        assert_eq!(company.industry, "Finance & Insurance");
        assert_eq!(company.jurisdiction, "US");
        assert_eq!(cases[0].resolutions[0].status, ResolutionStatus::Active);
    }

    #[tokio::test]
    async fn importing_twice_updates_instead_of_duplicating() {
        let repo = db::SqliteCaseRepository::connect("sqlite::memory:")
            .await
            .unwrap();
        let records = vec![
            dp_record("Acme Bank", "2020-01-01"),
            dp_record("Other Corp", "2023-01-01"),
        ];

        for _ in 0..2 {
            let (cases, _) = build_cases(&records, &ImportOptions::default(), today());
            for case in &cases {
                db::CaseRepository::upsert(&repo, case).await.unwrap();
            }
        }

        let all = db::CaseRepository::list(&repo).await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
