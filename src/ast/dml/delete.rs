use crate::ast::{types::TableName, DMLStatement, SQLStatement};

use super::parts::{_where::WhereClause, target::UpdateTarget};

#[derive(Clone, Debug, PartialEq, Default)]
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

#[cfg(test)]
mod tests {
    use crate::ast::types::SQLExpression;

    use super::*;

    #[test]
    fn test_builder_all() {
        let delete_query = DeleteQuery::builder()
            .set_from_table(TableName::new(None, "table".into()))
            .set_where(WhereClause {
                expression: SQLExpression::String("a".into()),
            })
            .set_from_alias("alias".into())
            .build();

        assert_eq!(
            delete_query,
            DeleteQuery {
                from_table: Some(UpdateTarget {
                    table: TableName::new(None, "table".into()),
                    alias: Some("alias".into()),
                }),
                where_clause: Some(WhereClause {
                    expression: SQLExpression::String("a".into()),
                }),
            }
        );
    }
}
