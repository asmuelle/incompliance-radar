use super::common::fetch_rss_with_html_bodies;
use crate::{CrawlerError, FilingSource, RawFiling};
use async_trait::async_trait;

const FEED_URL: &str = "https://www.sec.gov/news/pressreleases.rss";
/// SEC press release pages are Drupal-rendered; the body lives in this field.
const BODY_SELECTOR: &str = "div.field--name-body";
/// Caps LLM extraction calls per run and keeps us from re-fetching a
/// regulator's entire feed history — bump if a wider backlog is needed.
const MAX_ITEMS: usize = 10;

/// SEC press releases (`sec.gov/news/pressreleases.rss`).
///
/// Per the SEC's developer fair-access policy
/// (<https://www.sec.gov/os/webmaster-faq#developers>), automated requests
/// must carry a descriptive `User-Agent` identifying the application —
/// customize `user_agent` for your deployment rather than reusing the
/// default verbatim.
pub struct SecPressReleases {
    client: reqwest::Client,
    user_agent: String,
}

impl SecPressReleases {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            user_agent: user_agent.into(),
        }
    }
}

#[async_trait]
impl FilingSource for SecPressReleases {
    fn name(&self) -> &'static str {
        "sec"
    }

    async fn fetch_recent(&self) -> Result<Vec<RawFiling>, CrawlerError> {
        fetch_rss_with_html_bodies(
            &self.client,
            &self.user_agent,
            FEED_URL,
            BODY_SELECTOR,
            self.name(),
            MAX_ITEMS,
        )
        .await
    }
}
