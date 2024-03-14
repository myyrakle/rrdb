use crate::ast::predule::{SQLExpression, TableName};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
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
