use crate::lib::DataTypes;

#[derive(Clone, Debug)]
pub struct Column {
    pub name: String,
    pub data_type: DataTypes,
    pub comment: String,
    pub default: String,
    pub not_null: bool,
}
