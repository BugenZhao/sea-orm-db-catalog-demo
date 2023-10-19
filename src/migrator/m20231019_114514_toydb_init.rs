use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "toydb_init"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the Bakery table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let create_object = Table::create()
            .table(MyObject::Table)
            .col(
                ColumnDef::new(MyObject::Id)
                    .integer()
                    .not_null()
                    .auto_increment()
                    .primary_key(),
            )
            .col(ColumnDef::new(MyObject::Type).string().not_null())
            .col(ColumnDef::new(MyObject::Name).string().not_null())
            .col(ColumnDef::new(MyObject::DatabaseId).integer().not_null())
            .index(
                Index::create()
                    .col(MyObject::DatabaseId)
                    .col(MyObject::Type)
                    .col(MyObject::Name)
                    .unique(),
            )
            .to_owned();

        let create_database = Table::create()
            .table(MyDatabase::Table)
            .col(
                ColumnDef::new(MyDatabase::Id)
                    .integer()
                    .not_null()
                    .auto_increment()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(MyDatabase::Name)
                    .string()
                    .not_null()
                    .unique_key(),
            )
            .to_owned();

        let create_table = Table::create()
            .table(MyTable::Table)
            .col(
                ColumnDef::new(MyTable::ObjectId)
                    .integer()
                    .not_null()
                    .primary_key(),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_table_object_id")
                    .from(MyTable::Table, MyTable::ObjectId)
                    .to(MyObject::Table, MyObject::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned();

        let create_view = Table::create()
            .table(MyView::Table)
            .col(
                ColumnDef::new(MyView::ObjectId)
                    .integer()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(MyView::Definition).string().not_null())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_view_object_id")
                    .from(MyView::Table, MyView::ObjectId)
                    .to(MyObject::Table, MyObject::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned();

        let create_column = Table::create()
            .table(MyColumn::Table)
            .col(
                ColumnDef::new(MyColumn::Id)
                    .integer()
                    .not_null()
                    .auto_increment()
                    .primary_key(),
            )
            .col(ColumnDef::new(MyColumn::TableId).integer().not_null())
            .col(ColumnDef::new(MyColumn::Name).string().not_null())
            .col(ColumnDef::new(MyColumn::DataType).string().not_null())
            .col(ColumnDef::new(MyColumn::IsPrimaryKey).boolean().not_null())
            .index(
                Index::create()
                    .col(MyColumn::TableId)
                    .col(MyColumn::Name)
                    .unique(),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_column_table_id")
                    .from(MyColumn::Table, MyColumn::TableId)
                    .to(MyTable::Table, MyTable::ObjectId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned();

        let create_view_dependency = Table::create()
            .table(MyViewDependency::Table)
            .col(
                ColumnDef::new(MyViewDependency::ViewId)
                    .integer()
                    .not_null(),
            )
            .col(
                ColumnDef::new(MyViewDependency::DependentObjectId)
                    .integer()
                    .not_null(),
            )
            .primary_key(
                Index::create()
                    .col(MyViewDependency::ViewId)
                    .col(MyViewDependency::DependentObjectId),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_view_dependency_view_id")
                    .from(MyViewDependency::Table, MyViewDependency::ViewId)
                    .to(MyView::Table, MyView::ObjectId)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_view_dependency_dependenct_object_id")
                    .from(MyViewDependency::Table, MyViewDependency::DependentObjectId)
                    .to(MyObject::Table, MyObject::Id)
                    .on_delete(ForeignKeyAction::Restrict)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned();

        for table in vec![
            create_object,
            create_database,
            create_table,
            create_view,
            create_column,
            create_view_dependency,
        ] {
            manager.create_table(table).await?;
        }

        Ok(())
    }

    // Define how to rollback this migration: Drop the Bakery table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in vec![
            MyViewDependency::Table.into_table_ref(),
            MyColumn::Table.into_table_ref(),
            MyView::Table.into_table_ref(),
            MyTable::Table.into_table_ref(),
            MyDatabase::Table.into_table_ref(),
            MyObject::Table.into_table_ref(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }

        Ok(())
    }
}

#[derive(Iden)]
pub enum MyObject {
    Table,
    Id,
    Type,
    Name,
    DatabaseId,
}

#[derive(Iden)]
pub enum MyDatabase {
    Table,
    Id,
    Name,
}

#[derive(Iden)]
pub enum MyTable {
    Table,
    ObjectId,
}

#[derive(Iden)]
pub enum MyView {
    Table,
    ObjectId,
    Definition,
}

#[derive(Iden)]
pub enum MyColumn {
    Table,
    TableId,
    Id,
    Name,
    DataType,
    IsPrimaryKey,
}

#[derive(Iden)]
pub enum MyViewDependency {
    Table,
    ViewId,
    DependentObjectId,
}