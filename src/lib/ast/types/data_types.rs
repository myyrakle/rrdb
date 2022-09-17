use serde::{Deserialize, Serialize};

// SQL 데이터 타입
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Boolean,
    Varchar(i64),
}

impl From<DataType> for String {
    fn from(value: DataType) -> Self {
        match value {
            DataType::Int => "integer".into(),
            DataType::Float => "float".into(),
            DataType::Boolean => "boolean".into(),
            DataType::Varchar(number) => format!("varchar({})", number),
        }
    }
}
