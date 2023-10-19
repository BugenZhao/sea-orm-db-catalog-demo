use sea_orm_migration::prelude::*;

mod m20231019_114514_toydb_init;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20231019_114514_toydb_init::Migration)]
    }
}
