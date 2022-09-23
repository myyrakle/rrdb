use serde::{Deserialize, Serialize};

// SQL 데이터 타입
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Boolean,
    Varchar(i64),
}

impl DataType {
    pub fn type_code(&self) -> isize {
        match self {
            DataType::Int => 1,
            DataType::Float => 2,
            DataType::Boolean => 3,
            DataType::Varchar(_) => 4,
        }
    }
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
