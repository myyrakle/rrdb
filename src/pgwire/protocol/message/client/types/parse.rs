use crate::lib::pgwire::protocol::DataTypeOid;

#[derive(Debug)]
pub struct Parse {
    pub prepared_statement_name: String,
    pub query: String,
    pub parameter_types: Vec<DataTypeOid>,
}
