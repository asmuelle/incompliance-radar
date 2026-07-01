use crate::html::extract_text;
use crate::{CrawlerError, RawFiling};
use std::time::Duration;

/// Politeness delay between individual press-release page fetches, so a
/// crawl run doesn't hammer a regulator's site with a burst of requests (SEC
/// in particular enforces a documented per-IP rate limit and will start
/// returning 403s if you go faster than this).
const REQUEST_DELAY: Duration = Duration::from_secs(1);

/// Shared implementation for the common "RSS feed of links + per-item HTML
/// page with body text in one CSS-selectable container" shape both SEC and
/// FCA happen to use. A future source with a genuinely different shape (a
/// JSON API, paginated HTML listing, etc.) should implement `FilingSource`
/// directly instead of trying to force it through this helper.
pub(crate) async fn fetch_rss_with_html_bodies(
    client: &reqwest::Client,
    user_agent: &str,
    feed_url: &str,
    body_selector: &str,
    source: &'static str,
    max_items: usize,
) -> Result<Vec<RawFiling>, CrawlerError> {
    let feed_bytes = client
        .get(feed_url)
        .header("User-Agent", user_agent)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let channel = rss::Channel::read_from(&feed_bytes[..])?;

    let mut filings = Vec::new();
    for item in channel.items().iter().take(max_items) {
        let (Some(url), Some(title)) = (item.link(), item.title()) else {
            continue;
        };

        let published_on = item
            .pub_date()
            .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
            .map(|d| d.date_naive());

        match fetch_body(client, user_agent, url, body_selector).await {
            Ok(Some(text)) => filings.push(RawFiling {
                source,
                url: url.to_string(),
                title: title.trim().to_string(),
                published_on,
                text,
            }),
            Ok(None) => tracing::warn!(%url, %body_selector, "selector matched no body text"),
            Err(err) if is_rate_limited(&err) => {
                tracing::warn!(%url, %err, "rate limited, stopping this source's fetch early");
                break;
            }
            Err(err) => tracing::warn!(%url, %err, "failed to fetch press release page"),
        }

        tokio::time::sleep(REQUEST_DELAY).await;
    }

    Ok(filings)
}

async fn fetch_body(
    client: &reqwest::Client,
    user_agent: &str,
    url: &str,
    body_selector: &str,
) -> Result<Option<String>, CrawlerError> {
    let html = client
        .get(url)
        .header("User-Agent", user_agent)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let text = extract_text(&html, body_selector);
    Ok((!text.is_empty()).then_some(text))
}

fn is_rate_limited(err: &CrawlerError) -> bool {
    matches!(err, CrawlerError::Fetch(e) if matches!(e.status().map(|s| s.as_u16()), Some(403) | Some(429)))
}
