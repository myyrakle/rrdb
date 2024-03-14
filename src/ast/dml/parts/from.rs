use crate::ast::predule::{SubqueryExpression, TableName};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct FromClause {
    pub from: FromTarget,
    pub alias: Option<String>,
}

impl FromClause {}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum FromTarget {
    Table(TableName),             // 일반 테이블 참조
    Subquery(SubqueryExpression), // 서브쿼리 참조
}

impl FromTarget {}
