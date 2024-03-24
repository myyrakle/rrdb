use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Function {
    BuiltIn(BuiltInFunction),         // 내장함수
    UserDefined(UserDefinedFunction), // 사용자 정의 함수
}

impl Function {
    pub fn is_aggregate(&self) -> bool {
        match self {
            Self::BuiltIn(built_in) => match built_in {
                BuiltInFunction::Aggregate(_) => true,
                BuiltInFunction::Conditional(_) => false,
            },
            Self::UserDefined(_) => false,
        }
    }
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

impl TryFrom<String> for BuiltInFunction {
    type Error = ();

    fn try_from(function_name: String) -> Result<BuiltInFunction, Self::Error> {
        match function_name.to_uppercase().as_str() {
            "SUM" => Ok(AggregateFunction::Sum.into()),
            "COUNT" => Ok(AggregateFunction::Count.into()),
            "MAX" => Ok(AggregateFunction::Max.into()),
            "MIN" => Ok(AggregateFunction::Min.into()),
            "AVG" => Ok(AggregateFunction::Avg.into()),
            "EVERY" => Ok(AggregateFunction::Every.into()),
            "ARRAYAGG" => Ok(AggregateFunction::ArrayAgg.into()),
            "STRINGAGG" => Ok(AggregateFunction::StringAgg.into()),
            "NULLIF" => Ok(ConditionalFunction::NullIf.into()),
            "COALESCE" => Ok(ConditionalFunction::Coalesce.into()),
            "GREATEST" => Ok(ConditionalFunction::Greatest.into()),
            "LEAST" => Ok(ConditionalFunction::Least.into()),
            _ => Err(()),
        }
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
