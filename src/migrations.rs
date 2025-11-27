use diesel::{Connection, PgConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tracing::info;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub fn run_migrations(url: &str) -> Result<(), anyhow::Error> {
    let mut db = PgConnection::establish(url)?;
    let migrations = db
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .len();

    info!("ran migrations: {migrations}");

    Ok(())
}
