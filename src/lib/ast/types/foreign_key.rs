use crate::lib::ast::predule::TableName;

#[derive(Clone, Debug, PartialEq)]
pub struct ForeignKey {
    pub key_name: String,
    pub table: TableName,
    pub columns: Vec<String>,
    pub referenced_table: TableName,
    pub referenced_columns: Vec<String>,
}
