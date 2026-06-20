use anyhow::{anyhow, Context, Result};
use diesel::pg::PgConnection;
use diesel::Connection;
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::{info, instrument};

/// Embedded migrations baked into the binary at compile time so `cargo run`
/// (and any deployed binary) can apply schema changes without an external
/// CLI being available at runtime.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Async connection pool used by request handlers.
pub type DbPool = Pool<AsyncPgConnection>;

/// Open the async pool used by every request handler.
#[instrument(skip(database_url), fields(url_len = database_url.len()))]
pub async fn init_pool(database_url: &str) -> Result<DbPool> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(config)
        .await
        .context("Failed to build database connection pool")?;
    info!("database pool initialised");
    Ok(pool)
}

/// Run any pending embedded migrations using a one-shot sync `PgConnection`.
///
/// We use sync diesel here (not diesel-async) because `MigrationHarness` is a
/// sync trait. The async pool would force us through `AsyncConnectionWrapper`
/// + `block_in_place`, which is more code than just opening a single sync
/// connection that lives for the duration of the migration run.
///
/// Blocking work is done inside `spawn_blocking` so we don't stall the tokio
/// runtime during boot.
#[instrument(skip(database_url), fields(url_len = database_url.len()))]
pub async fn run_pending_migrations(database_url: &str) -> Result<()> {
    let url = database_url.to_string();
    let applied = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
        let mut conn = PgConnection::establish(&url)
            .with_context(|| "Failed to open sync connection for migrations")?;
        let names = conn
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow!("Migration failed: {e}"))?;
        Ok(names.into_iter().map(|n| n.to_string()).collect())
    })
    .await
    .context("Migration task panicked")??;

    if applied.is_empty() {
        info!("no pending migrations");
    } else {
        info!(count = applied.len(), migrations = ?applied, "applied migrations");
    }
    Ok(())
}
