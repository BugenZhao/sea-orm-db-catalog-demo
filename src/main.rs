#[allow(clippy::enum_variant_names)] // `My` prefix is intentional
#[rustfmt::skip]
mod entities;
mod entities_extension;
mod migrator;
mod repl;
mod session;

use anyhow::Result;
use migrator::Migrator;
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::prelude::*;

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

#[cfg(test)]
mod tests {
    use expect_test::expect;
    use sea_orm::{sea_query::OnConflict, DatabaseBackend, EntityTrait, QueryTrait, Set};

    use super::entities::my_object;

    #[test]
    fn test() {
        let m = my_object::ActiveModel {
            id: Set(233),
            ..Default::default()
        };

        let sql = my_object::Entity::insert(m)
            .on_conflict(
                OnConflict::column(my_object::Column::Id)
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .build(DatabaseBackend::MySql);

        expect!["INSERT INTO `my_object` (`id`) VALUES (233) ON DUPLICATE KEY DO NOTHING"].assert_eq(&sql.to_string());
    }
}
