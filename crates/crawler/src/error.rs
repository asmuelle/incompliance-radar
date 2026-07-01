#[derive(Debug, thiserror::Error)]
pub enum CrawlerError {
    #[error("failed to fetch source: {0}")]
    Fetch(#[from] reqwest::Error),

    #[error("failed to parse feed: {0}")]
    Feed(#[from] rss::Error),

    #[error("repository error: {0}")]
    Repository(#[from] db::RepositoryError),
}
