use crate::lib::pgwire::protocol::DataTypeOid;

#[derive(Debug, Clone)]
pub struct FieldDescription {
    pub name: String,
    pub data_type: DataTypeOid,
}
