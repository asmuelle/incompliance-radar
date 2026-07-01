use super::common::fetch_rss_with_html_bodies;
use crate::{CrawlerError, FilingSource, RawFiling};
use async_trait::async_trait;

/// FCA's general news feed — there is no press-releases-only feed as of
/// writing (`/news/press-releases/rss.xml` 404s), so this includes other
/// news alongside enforcement press releases. `extraction::extract_case`'s
/// prompt already instructs the model to leave fields null rather than guess
/// when a filing isn't actually a DPA/NPA/monitorship, so non-enforcement
/// items just extract to a mostly-empty case rather than failing outright.
const FEED_URL: &str = "https://www.fca.org.uk/news/rss.xml";
/// FCA press release pages wrap the whole article (title, date, body) in
/// this element; a plain "article" selector is coarser than SEC's dedicated
/// body div but there's no more specific container in their markup.
const BODY_SELECTOR: &str = "article";
/// Caps LLM extraction calls per run and keeps us from re-fetching a
/// regulator's entire feed history — bump if a wider backlog is needed.
const MAX_ITEMS: usize = 10;

/// FCA news and press releases (`fca.org.uk/news/rss.xml`).
pub struct FcaNews {
    client: reqwest::Client,
    user_agent: String,
}

impl FcaNews {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            user_agent: user_agent.into(),
        }
    }
}

#[async_trait]
impl FilingSource for FcaNews {
    fn name(&self) -> &'static str {
        "fca"
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
