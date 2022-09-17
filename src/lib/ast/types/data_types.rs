use serde::{Deserialize, Serialize};

// SQL 데이터 타입
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Boolean,
    Varchar(i64),
}
