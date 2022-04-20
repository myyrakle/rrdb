// SQL 데이터 타입
#[derive(Clone, Debug)]
pub enum DataType {
    Int,
    Float,
    Boolean,
    Varchar(i64),
}
