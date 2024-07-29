use crate::ast::types::{SQLExpression, TableName};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub right: TableName,
    pub right_alias: Option<String>,
    pub on: Option<SQLExpression>,
}

impl JoinClause {}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum JoinType {
    InnerJoin,
    LeftOuterJoin,
    RightOuterJoin,
    FullOuterJoin,
}

impl Default for JoinType {
    fn default() -> Self {
        JoinType::InnerJoin
    }
}
