use serde::{Deserialize, Serialize};

// SQL 데이터 타입
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[derive(Default)]
pub enum DataType {
    #[default]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_type_code() {
        assert_eq!(DataType::Int.type_code(), 1);
        assert_eq!(DataType::Float.type_code(), 2);
        assert_eq!(DataType::Boolean.type_code(), 3);
        assert_eq!(DataType::Varchar(255).type_code(), 4);
    }

    #[test]
    fn test_data_type_into_string() {
        assert_eq!(String::from(DataType::Int), "integer");
        assert_eq!(String::from(DataType::Float), "float");
        assert_eq!(String::from(DataType::Boolean), "boolean");
        assert_eq!(String::from(DataType::Varchar(255)), "varchar(255)");
    }
}
