use crate::lib::Table;

#[derive(Clone, Debug)]
pub struct ForeignKey {
    pub key_name: String,
    pub table: Table,
    pub columns: Vec<String>,
    pub referenced_table: Table,
    pub referenced_columns: Vec<String>,
}
