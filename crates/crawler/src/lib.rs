//! Scheduled fetch jobs against public regulator sources, feeding raw filing
//! text into `crates/extraction` and persisting the result via
//! `db::CaseRepository`. Not scheduled by anything itself — invoke the
//! `crawl` binary (`src/bin/crawl.rs`) periodically via cron, a systemd
//! timer, or similar. See CLAUDE.md for why there's no scraper for the DoJ
//! (blocks automated clients with a bot-management challenge) and why only
//! SEC and FCA are implemented today.

mod error;
mod html;
pub mod sources;

pub use error::CrawlerError;

use async_trait::async_trait;
use chrono::NaiveDate;
use std::collections::HashSet;

/// One fetched filing (a press release, litigation release, etc.) with its
/// full body text, ready to hand to `extraction::extract_case`.
#[derive(Debug, Clone)]
pub struct RawFiling {
    pub source: &'static str,
    pub url: String,
    pub title: String,
    pub published_on: Option<NaiveDate>,
    pub text: String,
}

#[async_trait]
pub trait FilingSource: Send + Sync {
    fn name(&self) -> &'static str;

    /// Fetches recently published filings. Implementations decide how many /
    /// how far back — there's no `since` cursor yet; `run_crawl` dedupes
    /// against already-persisted filings by URL, so re-fetching the same
    /// window repeatedly is safe, just wasted network calls once a source's
    /// recent-items window is fully ingested.
    async fn fetch_recent(&self) -> Result<Vec<RawFiling>, CrawlerError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CrawlSummary {
    pub fetched: usize,
    pub skipped_existing: usize,
    /// Successfully extracted as "not an enforcement action" per
    /// `extraction::extract_case`'s `Ok(None)` — not a failure, most items in
    /// a general news feed are expected to land here.
    pub skipped_not_applicable: usize,
    pub extracted: usize,
    pub failed: usize,
}

/// Fetches recent filings from `source`, skips any whose URL is already
/// recorded as a resolution's `source` on an existing case, and extracts +
/// persists the rest via `provider` and `repo`.
///
/// A single filing failing to extract or persist is logged and counted in
/// `failed` rather than aborting the run — a malformed press release
/// shouldn't block ingesting the other 49 in the same batch.
pub async fn run_crawl(
    source: &dyn FilingSource,
    provider: &dyn llm::LlmProvider,
    repo: &dyn db::CaseRepository,
) -> Result<CrawlSummary, CrawlerError> {
    let filings = source.fetch_recent().await?;
    let existing = existing_source_urls(repo).await?;

    let mut summary = CrawlSummary {
        fetched: filings.len(),
        ..Default::default()
    };

    for filing in filings {
        if existing.contains(&filing.url) {
            summary.skipped_existing += 1;
            continue;
        }
        ingest_filing(&filing, provider, repo, &mut summary).await;
    }

    Ok(summary)
}

async fn existing_source_urls(
    repo: &dyn db::CaseRepository,
) -> Result<HashSet<String>, CrawlerError> {
    Ok(repo
        .list()
        .await?
        .into_iter()
        .flat_map(|case| case.resolutions.into_iter().filter_map(|r| r.source))
        .collect())
}

async fn ingest_filing(
    filing: &RawFiling,
    provider: &dyn llm::LlmProvider,
    repo: &dyn db::CaseRepository,
    summary: &mut CrawlSummary,
) {
    let mut case = match extraction::extract_case(provider, &filing.text).await {
        Ok(Some(case)) => case,
        Ok(None) => {
            tracing::debug!(url = %filing.url, "filing is not an enforcement action, skipping");
            summary.skipped_not_applicable += 1;
            return;
        }
        Err(err) => {
            tracing::warn!(url = %filing.url, %err, "failed to extract case from filing");
            summary.failed += 1;
            return;
        }
    };

    for resolution in &mut case.resolutions {
        resolution.source.get_or_insert_with(|| filing.url.clone());
    }

    match repo.upsert(&case).await {
        Ok(()) => summary.extracted += 1,
        Err(err) => {
            tracing::error!(url = %filing.url, %err, "failed to persist extracted case");
            summary.failed += 1;
        }
    }
}
