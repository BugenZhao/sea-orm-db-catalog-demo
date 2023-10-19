use sea_orm::ActiveValue::*;
use sqlparser::ast;

use crate::entities::my_column;

impl my_column::ActiveModel {
    pub fn from_ast(col: ast::ColumnDef, table_id: i32) -> Self {
        my_column::ActiveModel {
            id: NotSet, // auto increment
            name: Set(col.name.value.to_owned()),
            table_id: Set(table_id),
            data_type: Set(col.data_type.to_string()),
            is_primary_key: Set(col
                .options
                .iter()
                .any(|c| matches!(c.option, ast::ColumnOption::Unique { is_primary: true }))),
        }
    }
}
