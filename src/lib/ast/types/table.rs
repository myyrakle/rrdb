// [database_name.]table_name
// 테이블명을 가리키는 값입니다.
#[derive(Clone, Debug)]
pub struct TableName {
    pub database_name: Option<String>,
    pub table_name: String,
}

impl TableName {
    pub fn new(database_name: Option<String>, table_name: String) -> Self {
        TableName {
            database_name,
            table_name,
        }
    }
}
