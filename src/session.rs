use std::ops::ControlFlow;

use anyhow::{bail, Context, Result};
use sea_orm::ActiveValue::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    QueryFilter, TransactionTrait,
};
use sqlparser::ast::{self, visit_relations};

use crate::entities::prelude::*;
use crate::entities::*;

pub struct Session {
    meta: DatabaseConnection,
    current_db: Option<my_database::Model>,
}

impl Session {
    pub fn new(meta: DatabaseConnection) -> Self {
        Self {
            meta,
            current_db: None,
        }
    }

    pub async fn handle(&mut self, stmt: ast::Statement) -> Result<()> {
        use ast::Statement::*;

        match stmt {
            CreateDatabase { db_name, .. } => self.create_database(db_name).await?,
            Use { db_name } => self.use_database(db_name).await?,

            CreateTable { name, columns, .. } => self.create_table(name, columns).await?,
            Drop {
                object_type: ast::ObjectType::Table,
                names,
                ..
            } => self.drop_object("table", names).await?,

            AlterTable {
                name, operations, ..
            } => self.alter_table(name, operations).await?,

            ShowTables { .. } => self.show_tables().await?,
            ExplainTable { table_name, .. } => self.explain_table(table_name).await?,

            CreateView { name, query, .. } => self.create_view(name, query).await?,
            Drop {
                object_type: ast::ObjectType::View,
                names,
                ..
            } => self.drop_object("view", names).await?,

            _ => bail!("unimplemented statement"),
        }

        Ok(())
    }

    fn current_db_id(&self) -> Result<i32> {
        self.current_db
            .as_ref()
            .map(|db| db.id)
            .context("no database selected")
            .map_err(Into::into)
    }

    pub fn current_db_name(&self) -> Option<&str> {
        self.current_db.as_ref().map(|db| db.name.as_str())
    }

    async fn create_database(&mut self, db_name: ast::ObjectName) -> Result<()> {
        let db_name = &db_name.0[0].value;
        let my_db = my_database::ActiveModel {
            id: NotSet,
            name: Set(db_name.to_owned()),
        };

        let db = my_db.insert(&self.meta).await?;

        if self.current_db.is_none() {
            self.current_db = Some(db);
        }

        Ok(())
    }

    async fn use_database(&mut self, db_name: ast::Ident) -> Result<()> {
        let db = MyDatabase::find()
            // Really confusing here, especially if there's no `my_` prefix.
            // Given a SQL term, you cannot tell if it's talking about our database or the meta
            // store.
            //
            // `my_database`: entity (table) name
            // `Column`: generated enum for all columns
            // `Name`: column named `name`
            .filter(my_database::Column::Name.eq(&db_name.value))
            .one(&self.meta)
            .await?
            .context("database not found")?;

        self.current_db = Some(db);

        Ok(())
    }

    #[allow(dead_code)]
    async fn drop_database(&mut self, db_name: ast::Ident) -> Result<()> {
        let res = MyDatabase::delete_many()
            .filter(my_database::Column::Name.eq(&db_name.value))
            .exec(&self.meta)
            .await?;

        if res.rows_affected == 0 {
            bail!("database `{}` not found", db_name.value);
        }

        if let Some(current_db) = &self.current_db {
            if current_db.name == db_name.value {
                self.current_db = None;
            }
        }

        Ok(())
    }

    async fn create_table(
        &mut self,
        table_name: ast::ObjectName,
        columns: Vec<ast::ColumnDef>,
    ) -> Result<()> {
        let db_id = self.current_db_id()?;

        let table_name = &table_name.0[0].value;

        let txn = self.meta.begin().await?;

        let my_object = my_object::ActiveModel {
            id: NotSet,
            name: Set(table_name.to_owned()),
            r#type: Set("table".to_owned()),
            database_id: Set(db_id),
        };
        let object_id = my_object.insert(&txn).await?.id;

        let my_table = my_table::ActiveModel {
            object_id: Set(object_id),
        };
        my_table.insert(&txn).await?;

        let my_columns = columns
            .into_iter()
            .map(|col| my_column::ActiveModel::from_ast(col, object_id))
            .collect::<Vec<_>>();

        MyColumn::insert_many(my_columns).exec(&txn).await?;

        txn.commit().await?;

        Ok(())
    }

    async fn show_tables(&mut self) -> Result<()> {
        let db_id = self.current_db_id()?;

        let tables = MyObject::find()
            .filter(
                (my_object::Column::DatabaseId.eq(db_id)).and(my_object::Column::Type.eq("table")),
            )
            .all(&self.meta)
            .await?;

        for table in tables {
            println!("{}", table.name);
        }

        Ok(())
    }

    async fn explain_table(&mut self, table_name: ast::ObjectName) -> Result<()> {
        let db_id = self.current_db_id()?;

        let table_name = &table_name.0[0].value;

        let txn = self.meta.begin().await?;

        let table = MyTable::find()
            .inner_join(MyObject) // so that we can filter by columns from `my_object`
            .filter(
                (my_object::Column::DatabaseId.eq(db_id))
                    .and(my_object::Column::Type.eq("table"))
                    .and(my_object::Column::Name.eq(table_name)),
            )
            .one(&txn)
            .await?
            .context("table not found")?;

        // Why not eager loading like `alter_table`? Just for demo purpose.
        let columns = table.find_related(MyColumn).all(&txn).await?;

        for column in columns {
            println!(
                "{}\t{}\t{}",
                column.name,
                column.data_type,
                if column.is_primary_key { "PRI" } else { "" }
            );
        }

        Ok(())
    }

    async fn alter_table(
        &mut self,
        table_name: ast::ObjectName,
        operations: Vec<ast::AlterTableOperation>,
    ) -> Result<()> {
        let db_id = self.current_db_id()?;

        let table_name = &table_name.0[0].value;

        let txn = self.meta.begin().await?;

        let (table, columns) = MyTable::find()
            .inner_join(MyObject)
            .filter(
                (my_object::Column::DatabaseId.eq(db_id))
                    .and(my_object::Column::Type.eq("table"))
                    .and(my_object::Column::Name.eq(table_name)),
            )
            .find_with_related(MyColumn) // eager loading, compared to `find_related` in `explain_table`
            .all(&txn)
            .await?
            .into_iter()
            .next()
            .context("table not found")?;

        for op in operations {
            use ast::AlterTableOperation::*;

            // TODO: inefficient to manipulate columns one by one
            match op {
                AddColumn { column_def, .. } => {
                    let column = my_column::ActiveModel::from_ast(column_def, table.object_id);
                    MyColumn::insert(column).exec(&txn).await?;
                }
                DropColumn { column_name, .. } => {
                    let column = columns
                        .iter()
                        .find(|c| c.name == column_name.value)
                        .context("column not found")?
                        .clone()
                        .into_active_model();

                    MyColumn::delete(column).exec(&txn).await?;
                }
                _ => bail!("unimplemented alter table operation"),
            }
        }

        txn.commit().await?;

        Ok(())
    }

    async fn create_view(
        &mut self,
        view_name: ast::ObjectName,
        query: Box<ast::Query>,
    ) -> Result<()> {
        let db_id = self.current_db_id()?;

        let view_name = &view_name.0[0].value;

        let mut references = Vec::new();
        visit_relations(&query, |r| {
            references.push(r.0[0].value.to_owned());
            ControlFlow::<()>::Continue(())
        });

        let txn = self.meta.begin().await?;

        let mut reference_ids = Vec::new();
        for reference in references {
            let id = MyObject::find()
                .filter(my_object::Column::Name.eq(reference))
                .one(&txn)
                .await?
                .context("referenced object not found")?
                .id;

            reference_ids.push(id);
        }

        let my_object = my_object::ActiveModel {
            id: NotSet,
            name: Set(view_name.to_owned()),
            r#type: Set("view".to_owned()),
            database_id: Set(db_id),
        };
        let object_id = my_object.insert(&txn).await?.id;

        let my_view = my_view::ActiveModel {
            object_id: Set(object_id),
            definition: Set(query.to_string()),
        };
        my_view.insert(&txn).await?;

        let my_view_references = reference_ids
            .into_iter()
            .map(|table_id| my_view_dependency::ActiveModel {
                view_id: Set(object_id),
                dependent_object_id: Set(table_id),
            })
            .collect::<Vec<_>>();
        MyViewDependency::insert_many(my_view_references)
            .exec(&txn)
            .await?;

        txn.commit().await?;

        Ok(())
    }

    async fn drop_object(&mut self, object_type: &str, names: Vec<ast::ObjectName>) -> Result<()> {
        let db_id = self.current_db_id()?;

        let txn = self.meta.begin().await?;

        for name in names {
            let name = &name.0[0].value;

            // Constraints will help us restrict or cascade the deletion.
            let res = MyObject::delete_many()
                .filter(
                    (my_object::Column::Name.eq(name.as_str()))
                        .and(my_object::Column::Type.eq(object_type))
                        .and(my_object::Column::DatabaseId.eq(db_id)),
                )
                .exec(&txn)
                .await?;

            if res.rows_affected == 0 {
                bail!("{object_type} `{name}` not found");
            }
        }

        txn.commit().await?;

        Ok(())
    }
}
