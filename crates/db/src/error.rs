#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("failed to run migrations: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("failed to (de)serialize case data: {0}")]
    Serialization(#[from] serde_json::Error),
}
