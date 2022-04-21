pub use crate::lib::ast::traits::{DDLStatement, SQLStatement};
pub use crate::lib::ast::types::Column;
use crate::lib::{ForeignKey, Table, TableOptions, UniqueKey};

/*
CREATE TABLE [IF NOT EXISTS] [database_name.]table_name (
    column_name data_type [NOT NULL | NULL] [PRIMARY KEY] [COMMENT 'comment'],
    column_name data_type [NOT NULL | NULL] [PRIMARY KEY] [COMMENT 'comment'],
    ...
    PRIMARY KEY (column_name),
    UNIQUE (column_name),
    FOREIGN KEY (column_name) REFERENCES table_name (column_name),
    FOREIGN KEY (column_name) REFERENCES table_name (column_name),
    ...
);
*/

#[derive(Debug, Clone)]
pub struct CreateTableQuery {
    pub table: Option<Table>,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub unique_keys: Vec<UniqueKey>,
    pub table_options: Option<TableOptions>,
    pub if_not_exists: bool,
}

impl CreateTableQuery {
    pub fn builder() -> Self {
        CreateTableQuery {
            table: None,
            columns: vec![],
            primary_key: vec![],
            foreign_keys: vec![],
            unique_keys: vec![],
            table_options: None,
            if_not_exists: false,
        }
    }

    pub fn set_table<'a>(&'a mut self, table: Table) -> &'a mut Self {
        self.table = Some(table);
        self
    }

    pub fn set_table_option<'a>(&'a mut self, option: TableOptions) -> &'a mut Self {
        self.table_options = Some(option);
        self
    }

    pub fn add_column<'a>(&'a mut self, column: Column) -> &'a mut Self {
        self.columns.push(column);
        self
    }

    pub fn set_primary_key<'a>(&'a mut self, columns: Vec<String>) -> &'a mut Self {
        self.primary_key = columns;
        self
    }

    pub fn add_unique_key<'a>(&'a mut self, unique_key: UniqueKey) -> &'a mut Self {
        self.unique_keys.push(unique_key);
        self
    }

    pub fn set_if_not_exists<'a>(&'a mut self, if_not_exists: bool) -> &'a mut Self {
        self.if_not_exists = if_not_exists;
        self
    }

    pub fn build(self) -> Box<dyn SQLStatement> {
        Box::new(self)
    }
}

impl DDLStatement for CreateTableQuery {}

impl SQLStatement for CreateTableQuery {}
