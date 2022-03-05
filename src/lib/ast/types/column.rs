use crate::lib::DataTypes;

#[derive(Clone, Debug)]
pub struct Column {
    name: String,
    dataType: DataTypes,
    comment: String,
    default: String,
    notNull: bool,
}
