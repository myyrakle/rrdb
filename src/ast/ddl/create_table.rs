use crate::ast::{
    types::{Column, ForeignKey, TableName, TableOptions, UniqueKey},
    DDLStatement, SQLStatement,
};

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

#[derive(Clone, Debug, PartialEq)]
pub struct CreateTableQuery {
    pub table: Option<TableName>,
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

    pub fn set_table(mut self, table: TableName) -> Self {
        self.table = Some(table);
        self
    }

    pub fn set_table_option(mut self, option: TableOptions) -> Self {
        self.table_options = Some(option);
        self
    }

    pub fn add_column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    pub fn set_primary_key(mut self, columns: Vec<String>) -> Self {
        self.primary_key = columns;
        self
    }

    pub fn add_unique_key(mut self, unique_key: UniqueKey) -> Self {
        self.unique_keys.push(unique_key);
        self
    }

    pub fn set_if_not_exists(mut self, if_not_exists: bool) -> Self {
        self.if_not_exists = if_not_exists;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::CreateTableQuery(self))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::types::DataType;

    use super::*;

    #[test]
    fn test_create_table() {
        let query = CreateTableQuery::builder()
            .set_table(TableName::new(None, "table_name".into()))
            .add_column(
                Column::builder()
                    .set_name("column_name".into())
                    .set_data_type(DataType::Int)
                    .set_not_null(true)
                    .set_primary_key(true)
                    .set_comment("comment".into())
                    .build(),
            )
            .set_primary_key(vec!["column_name".into()])
            .set_table_option(TableOptions {})
            .add_unique_key(UniqueKey {
                key_name: "unique_key".into(),
                database_name: None,
                columns: vec!["column_name".into()],
            })
            .set_if_not_exists(true)
            .build();

        let expected = SQLStatement::DDL(DDLStatement::CreateTableQuery(CreateTableQuery {
            table: Some(TableName::new(None, "table_name".into())),
            columns: vec![Column {
                name: "column_name".into(),
                data_type: DataType::Int,
                comment: "comment".into(),
                default: None,
                not_null: true,
                primary_key: true,
            }],
            primary_key: vec!["column_name".into()],
            foreign_keys: vec![],
            unique_keys: vec![UniqueKey {
                key_name: "unique_key".into(),
                database_name: None,
                columns: vec!["column_name".into()],
            }],
            table_options: Some(TableOptions {}),
            if_not_exists: true,
        }));

        assert_eq!(query, expected);
    }
}
