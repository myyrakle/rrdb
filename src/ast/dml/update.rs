use crate::ast::predule::{DMLStatement, SQLStatement, TableName, UpdateItem, WhereClause};

use super::UpdateTarget;

#[derive(Clone, Debug, PartialEq)]
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
