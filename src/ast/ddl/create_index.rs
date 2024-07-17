use crate::ast::{
    types::{Column, TableName},
    DDLStatement, SQLStatement,
};

/*
CREATE [ UNIQUE ] INDEX [ IF NOT EXISTS ] name ON table_name
    ( column_name [, ...] )
*/

#[derive(Clone, Debug, PartialEq)]
pub struct CreateIndexQuery {
    pub index_name: String,
    pub table: TableName,
    pub columns: Vec<Column>,
    pub is_unique: bool,
    pub if_not_exists: bool,
}

impl CreateIndexQuery {
    pub fn builder() -> Self {
        Self {
            table: Default::default(),
            columns: vec![],
            is_unique: false,
            if_not_exists: false,
            index_name: "".into(),
        }
    }

    pub fn set_table(mut self, table: TableName) -> Self {
        self.table = table;
        self
    }

    pub fn set_index_name(mut self, index_name: String) -> Self {
        self.index_name = index_name;
        self
    }

    pub fn add_column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    pub fn set_unique(mut self, unique: bool) -> Self {
        self.is_unique = unique;
        self
    }

    pub fn set_if_not_exists(mut self, if_not_exists: bool) -> Self {
        self.if_not_exists = if_not_exists;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::CreateIndexQuery(self))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::types::DataType;

    use super::*;

    #[test]
    fn test_create_index() {
        let query = CreateIndexQuery::builder()
            .set_table(TableName::new(None, "table_name".into()))
            .set_index_name("index_name".into())
            .add_column(
                Column::builder()
                    .set_name("column_name".into())
                    .set_data_type(DataType::Boolean)
                    .build(),
            )
            .set_unique(true)
            .set_if_not_exists(true)
            .build();

        let expected = SQLStatement::DDL(DDLStatement::CreateIndexQuery(CreateIndexQuery {
            table: TableName::new(None, "table_name".into()),
            index_name: "index_name".into(),
            columns: vec![Column::builder()
                .set_name("column_name".into())
                .set_data_type(DataType::Boolean)
                .build()],
            is_unique: true,
            if_not_exists: true,
        }));

        assert_eq!(query, expected);
    }
}
