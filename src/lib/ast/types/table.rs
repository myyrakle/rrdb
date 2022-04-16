#[derive(Clone, Debug)]
pub struct Table {
    pub database_name: Option<String>,
    pub table_name: String,
}

impl Table {
    pub fn new(database_name: Option<String>, table_name: String) -> Self {
        Table {
            database_name,
            table_name,
        }
    }
}
