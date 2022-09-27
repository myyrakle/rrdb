use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Function {
    BuiltIn(BuiltInFunction),         // 내장함수
    UserDefined(UserDefinedFunction), // 사용자 정의 함수
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum BuiltInFunction {
    Aggregate(AggregateFunction),
    Conditional(ConditionalFunction),
}

impl From<BuiltInFunction> for Function {
    fn from(value: BuiltInFunction) -> Function {
        Function::BuiltIn(value)
    }
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

impl From<AggregateFunction> for BuiltInFunction {
    fn from(value: AggregateFunction) -> BuiltInFunction {
        BuiltInFunction::Aggregate(value)
    }
}

impl From<AggregateFunction> for Function {
    fn from(value: AggregateFunction) -> Function {
        BuiltInFunction::Aggregate(value).into()
    }
}

// 집합 함수
// 참고 https://www.postgresql.org/docs/9.5/functions-aggregate.html
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum ConditionalFunction {
    NullIf,
    Coalesce,
    Greatest,
    Least,
}

impl From<ConditionalFunction> for BuiltInFunction {
    fn from(value: ConditionalFunction) -> BuiltInFunction {
        BuiltInFunction::Conditional(value)
    }
}

impl From<ConditionalFunction> for Function {
    fn from(value: ConditionalFunction) -> Function {
        BuiltInFunction::Conditional(value).into()
    }
}

// 함수명을 가리키는 값입니다.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct UserDefinedFunction {
    pub database_name: Option<String>,
    pub function_name: String,
}

impl From<UserDefinedFunction> for Function {
    fn from(value: UserDefinedFunction) -> Function {
        Function::UserDefined(value)
    }
}

impl UserDefinedFunction {}
