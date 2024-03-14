use crate::ast::predule::{DMLStatement, SQLStatement, TableName, UpdateTarget, WhereClause};

#[derive(Clone, Debug, PartialEq)]
pub struct DeleteQuery {
    pub from_table: Option<UpdateTarget>,
    pub where_clause: Option<WhereClause>,
}

impl DeleteQuery {
    pub fn builder() -> Self {
        Self {
            from_table: None,
            where_clause: None,
        }
    }

    pub fn set_from_table(mut self, from: TableName) -> Self {
        self.from_table = Some(from.into());
        self
    }

    pub fn set_from_alias(mut self, alias: String) -> Self {
        if self.from_table.is_some() {
            self.from_table = self.from_table.map(|mut e| {
                e.alias = Some(alias);
                e
            });
        }
        self
    }

    pub fn set_where(mut self, where_clause: WhereClause) -> Self {
        self.where_clause = Some(where_clause);
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

impl From<DeleteQuery> for SQLStatement {
    fn from(value: DeleteQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::DeleteQuery(value))
    }
}
