//! One crawl pass across all configured `FilingSource`s, then exits. Meant
//! to be invoked periodically by an external scheduler (cron, a systemd
//! timer, a scheduled CI job, ...) — this binary has no scheduling loop of
//! its own. See CLAUDE.md for env vars and an example crontab entry.

use crawler::sources::{FcaNews, SecPressReleases};
use crawler::{run_crawl, FilingSource};
use db::SqliteCaseRepository;

const DEFAULT_DATABASE_URL: &str = "sqlite://incompliance-radar.db?mode=rwc";
const DEFAULT_USER_AGENT: &str =
    "incomplianceRadar/0.1 (+https://github.com/asmuelle/incompliance-radar)";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let user_agent =
        std::env::var("CRAWLER_USER_AGENT").unwrap_or_else(|_| DEFAULT_USER_AGENT.to_string());

    let repo = SqliteCaseRepository::connect(&database_url)
        .await
        .unwrap_or_else(|e| panic!("failed to connect to database at {database_url}: {e}"));
    let provider =
        llm::provider_from_env().expect("failed to configure LLM provider (see .env.example)");

    let sources: Vec<Box<dyn FilingSource>> = vec![
        Box::new(SecPressReleases::new(user_agent.clone())),
        Box::new(FcaNews::new(user_agent.clone())),
    ];

    for source in &sources {
        tracing::info!(source = source.name(), "starting crawl");
        match run_crawl(source.as_ref(), provider.as_ref(), &repo).await {
            Ok(summary) => tracing::info!(source = source.name(), ?summary, "crawl finished"),
            Err(err) => tracing::error!(source = source.name(), %err, "crawl failed"),
        }
    }
}
