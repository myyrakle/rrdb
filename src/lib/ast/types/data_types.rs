// SQL 데이터 타입
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Boolean,
    Varchar(i64),
}
