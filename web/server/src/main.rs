use app::{shell, App};
use axum::Router;
use db::{AlertRepository, CaseRepository, SqliteCaseRepository};
use leptos::config::get_configuration;
use leptos::prelude::provide_context;
use leptos_axum::{generate_route_list, LeptosRoutes};
use std::sync::Arc;

const DEFAULT_DATABASE_URL: &str = "sqlite://incompliance-radar.db?mode=rwc";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let conf = get_configuration(None).expect("failed to read leptos configuration");
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let sqlite_repo = connect_and_seed().await;
    let alert_repo: Arc<dyn AlertRepository> = Arc::new(sqlite_repo.alert_repository());
    let repo: Arc<dyn CaseRepository> = Arc::new(sqlite_repo);

    let router = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let repo = repo.clone();
                let alert_repo = alert_repo.clone();
                move || {
                    provide_context(repo.clone());
                    provide_context(alert_repo.clone());
                }
            },
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {addr}: {e}"));
    tracing::info!("incomplianceRadar listening on http://{addr}");
    axum::serve(listener, router.into_make_service())
        .await
        .expect("server error");
}

/// Connects to the configured SQLite database (creating and migrating it if
/// needed) and, on first run against an empty database, loads the fictional
/// demo cases from `app::seed` so the UI has something to show.
async fn connect_and_seed() -> SqliteCaseRepository {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let repo = SqliteCaseRepository::connect(&database_url)
        .await
        .unwrap_or_else(|e| panic!("failed to connect to database at {database_url}: {e}"));

    match repo.list().await {
        Ok(cases) if cases.is_empty() => {
            for case in app::seed::seed_cases() {
                if let Err(err) = repo.upsert(&case).await {
                    tracing::error!(%err, "failed to seed demo compliance case");
                }
            }
            tracing::info!("seeded demo compliance cases into empty database");
        }
        Ok(_) => {}
        Err(err) => tracing::error!(%err, "failed to check for existing cases before seeding"),
    }

    repo
}
