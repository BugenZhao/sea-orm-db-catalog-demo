#[allow(clippy::enum_variant_names)] // `My` prefix is intentional
mod entities;
mod entities_extension;
mod migrator;
mod repl;
mod session;

use anyhow::Result;

use migrator::Migrator;
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;

use tracing_subscriber::{filter::LevelFilter, prelude::*};

fn main() {
    let filter = tracing_subscriber::filter::Targets::new().with_default(LevelFilter::ERROR);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run())
        .unwrap();
}

async fn run() -> Result<()> {
    let db = Database::connect("sqlite:./sqlite.db?mode=rwc").await?;

    Migrator::up(&db, None).await?; // this is idempotent and used in production

    #[cfg(disabled)]
    Migrator::refresh(&db).await?; // this will reset the database with `down` then `up`

    let mut session = session::Session::new(db.clone());

    repl::repl(&mut session).await?;

    db.close().await?;

    Ok(())
}
