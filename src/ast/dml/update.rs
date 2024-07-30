use crate::ast::{types::TableName, DMLStatement, SQLStatement};

use super::parts::{_where::WhereClause, target::UpdateTarget, update_item::UpdateItem};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct UpdateQuery {
    pub target_table: Option<UpdateTarget>,
    pub where_clause: Option<WhereClause>,
    pub update_items: Vec<UpdateItem>,
}

impl UpdateQuery {
    pub fn builder() -> Self {
        Self {
            update_items: vec![],
            target_table: None,
            where_clause: None,
        }
    }

    pub fn add_update_item(mut self, item: UpdateItem) -> Self {
        self.update_items.push(item);
        self
    }

    pub fn set_target_table(mut self, from: TableName) -> Self {
        self.target_table = Some(from.into());
        self
    }

    pub fn set_target_alias(mut self, alias: String) -> Self {
        if self.target_table.is_some() {
            self.target_table = self.target_table.map(|mut e| {
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

impl From<UpdateQuery> for SQLStatement {
    fn from(value: UpdateQuery) -> SQLStatement {
        SQLStatement::DML(DMLStatement::UpdateQuery(value))
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::types::SQLExpression;

    use super::*;

    #[test]
    fn test_builder_all() {
        let update_query = UpdateQuery::builder()
            .set_target_table(TableName::new(None, "table".into()))
            .add_update_item(UpdateItem {
                column: "a".into(),
                value: SQLExpression::String("b".into()),
            })
            .set_where(WhereClause {
                expression: SQLExpression::String("a".into()),
            })
            .set_target_alias("alias".into())
            .build();
        assert_eq!(
            update_query,
            UpdateQuery {
                target_table: Some(UpdateTarget {
                    table: TableName::new(None, "table".into()),
                    alias: Some("alias".into()),
                }),
                where_clause: Some(WhereClause {
                    expression: SQLExpression::String("a".into()),
                }),
                update_items: vec![UpdateItem {
                    column: "a".into(),
                    value: SQLExpression::String("b".into()),
                }],
            }
        );
    }
}
