use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Function {
    BuiltIn(BuiltInFunction),         // 내장함수
    UserDefined(UserDefinedFunction), // 사용자 정의 함수
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum BuiltInFunction {
    Aggregate(AggregateFunction),
}

// 집합 함수
// 참고 https://www.postgresql.org/docs/9.5/functions-aggregate.html
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum AggregateFunction {
    Sum,
    Count,
    Max,
    Min,
    Avg,
    Every,
    ArrayAgg,
    StringAgg,
}

// 함수명을 가리키는 값입니다.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct UserDefinedFunction {
    pub database_name: Option<String>,
    pub function_name: String,
}

impl UserDefinedFunction {}
