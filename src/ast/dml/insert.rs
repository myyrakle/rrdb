use crate::ast::predule::{DMLStatement, InsertValue, SQLStatement, SelectQuery, TableName};

#[derive(Clone, Debug, PartialEq)]
pub struct InsertQuery {
    pub into_table: Option<TableName>,
    pub columns: Vec<String>,
    pub data: InsertData,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InsertData {
    Select(Box<SelectQuery>),
    Values(Vec<InsertValue>),
    None,
}

impl InsertQuery {
    pub fn builder() -> Self {
        Self {
            columns: vec![],
            into_table: None,
            data: InsertData::None,
        }
    }

    pub fn set_into_table(mut self, from: TableName) -> Self {
        self.into_table = Some(from);
        self
    }

    pub fn set_columns(mut self, columns: Vec<String>) -> Self {
        self.columns = columns;
        self
    }

    pub fn set_values(mut self, values: Vec<InsertValue>) -> Self {
        self.data = InsertData::Values(values);
        self
    }

    pub fn set_select(mut self, select: SelectQuery) -> Self {
        self.data = InsertData::Select(Box::new(select));
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl From<InsertQuery> for SQLStatement {
    fn from(value: InsertQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::InsertQuery(value))
    }
}
