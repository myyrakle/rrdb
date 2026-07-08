use serde::{Deserialize, Serialize};

use crate::engine::ast::{DDLStatement, SQLStatement, types::TableName};

/*
DROP INDEX [IF EXISTS] index_name [ON [database_name.]table_name];
*/
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DropIndexQuery {
    pub index_name: String,
    /// 인덱스가 속한 데이터베이스명 (파서 컨텍스트 혹은 ON 절에서 결정)
    pub database_name: Option<String>,
    /// ON 절로 명시된 테이블 (선택사항)
    pub table: Option<TableName>,
    pub if_exists: bool,
}

impl DropIndexQuery {
    pub fn builder() -> Self {
        DropIndexQuery {
            index_name: "".into(),
            database_name: None,
            table: None,
            if_exists: false,
        }
    }

    pub fn set_index_name(mut self, index_name: String) -> Self {
        self.index_name = index_name;
        self
    }

    pub fn set_database_name(mut self, database_name: Option<String>) -> Self {
        self.database_name = database_name;
        self
    }

    pub fn set_table(mut self, table: TableName) -> Self {
        self.database_name = table.database_name.clone().or(self.database_name);
        self.table = Some(table);
        self
    }

    pub fn set_if_exists(mut self, if_exists: bool) -> Self {
        self.if_exists = if_exists;
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::DropIndexQuery(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop_index() {
        let query = DropIndexQuery::builder()
            .set_index_name("index_name".into())
            .set_table(TableName::new(Some("db".into()), "table_name".into()))
            .set_if_exists(true)
            .build();

        let expected = SQLStatement::DDL(DDLStatement::DropIndexQuery(DropIndexQuery {
            index_name: "index_name".into(),
            database_name: Some("db".into()),
            table: Some(TableName::new(Some("db".into()), "table_name".into())),
            if_exists: true,
        }));

        assert_eq!(query, expected);
    }
}
