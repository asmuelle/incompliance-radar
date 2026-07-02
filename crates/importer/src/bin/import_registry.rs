//! One-shot bulk import of the Corporate Prosecution Registry CSV export
//! into the case database. Download the data first (`just import-registry`
//! wraps both steps), then:
//!
//! ```text
//! import-registry <path/to/corp-crime.csv> [--dispositions=DP,NP,declination] [--dry-run]
//! ```
//!
//! `DATABASE_URL` selects the database, same default as the server and the
//! crawler. Re-running is safe — imported case IDs are deterministic, so an
//! updated registry export refreshes existing cases instead of duplicating
//! them. See CLAUDE.md's Importer section, including the licensing caveat on
//! commercial reuse of the registry data.

use importer::{import_registry_csv, ImportOptions};
use std::path::PathBuf;

const DEFAULT_DATABASE_URL: &str = "sqlite://incompliance-radar.db?mode=rwc";

const USAGE: &str =
    "usage: import-registry <corp-crime.csv> [--dispositions=DP,NP,declination] [--dry-run]";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (csv_path, options) = parse_args(std::env::args().skip(1)).unwrap_or_else(|err| {
        eprintln!("{err}\n{USAGE}");
        std::process::exit(2);
    });

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let repo = db::SqliteCaseRepository::connect(&database_url)
        .await
        .unwrap_or_else(|e| panic!("failed to connect to database at {database_url}: {e}"));

    let today = chrono::Utc::now().date_naive();
    tracing::info!(path = %csv_path.display(), dispositions = ?options.dispositions, dry_run = options.dry_run, "starting registry import");
    match import_registry_csv(&csv_path, &options, &repo, today).await {
        Ok(summary) => tracing::info!(?summary, "registry import finished"),
        Err(err) => {
            tracing::error!(%err, "registry import failed");
            std::process::exit(1);
        }
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<(PathBuf, ImportOptions), String> {
    let mut csv_path: Option<PathBuf> = None;
    let mut options = ImportOptions::default();

    for arg in args {
        if let Some(list) = arg.strip_prefix("--dispositions=") {
            let dispositions: Vec<String> = list
                .split(',')
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(str::to_string)
                .collect();
            if dispositions.is_empty() {
                return Err("--dispositions needs at least one value".to_string());
            }
            options.dispositions = dispositions;
        } else if arg == "--dry-run" {
            options.dry_run = true;
        } else if arg.starts_with("--") {
            return Err(format!("unknown flag: {arg}"));
        } else if csv_path.is_some() {
            return Err(format!("unexpected extra argument: {arg}"));
        } else {
            csv_path = Some(PathBuf::from(arg));
        }
    }

    let csv_path = csv_path.ok_or_else(|| "missing path to corp-crime.csv".to_string())?;
    Ok((csv_path, options))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> std::vec::IntoIter<String> {
        list.iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn parses_path_and_defaults() {
        let (path, options) = parse_args(args(&["data/corp-crime.csv"])).unwrap();
        assert_eq!(path, PathBuf::from("data/corp-crime.csv"));
        assert!(!options.dry_run);
        assert_eq!(options.dispositions, vec!["DP", "NP", "declination"]);
    }

    #[test]
    fn parses_dispositions_and_dry_run() {
        let (_, options) =
            parse_args(args(&["x.csv", "--dispositions=DP,plea", "--dry-run"])).unwrap();
        assert!(options.dry_run);
        assert_eq!(options.dispositions, vec!["DP", "plea"]);
    }

    #[test]
    fn rejects_missing_path_and_unknown_flags() {
        assert!(parse_args(args(&[])).is_err());
        assert!(parse_args(args(&["x.csv", "--nope"])).is_err());
        assert!(parse_args(args(&["x.csv", "extra.csv"])).is_err());
    }
}
