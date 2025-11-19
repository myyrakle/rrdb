use crate::engine::ast::types::{SQLExpression, TableName};

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
#[derive(Default)]
pub enum JoinType {
    #[default]
    InnerJoin,
    LeftOuterJoin,
    RightOuterJoin,
    FullOuterJoin,
}

