use crate::ast::{types::TableName, DMLStatement, SQLStatement};

use super::{parts::insert_values::InsertValue, select::SelectQuery};

#[derive(Clone, Debug, PartialEq, Default)]
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

impl Default for InsertData {
    fn default() -> Self {
        Self::None
    }
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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::ast::types::SQLExpression;

    use super::*;

    #[test]
    fn test_From_InsertQuery_for_SQLStatement() {
        let insert_query = InsertQuery::builder()
            .set_into_table(TableName::new(None, "table".into()))
            .set_columns(vec!["a".into(), "b".into()])
            .set_values(vec![InsertValue {
                list: vec![Some(SQLExpression::String("a".into()))],
            }])
            .build();

        assert_eq!(
            SQLStatement::from(insert_query),
            SQLStatement::DML(DMLStatement::InsertQuery(InsertQuery {
                into_table: Some(TableName::new(None, "table".into())),
                columns: vec!["a".into(), "b".into()],
                data: InsertData::Values(vec![InsertValue {
                    list: vec![Some(SQLExpression::String("a".into()))],
                }]),
            }))
        );
    }
}
