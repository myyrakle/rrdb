use bytes::BytesMut;

use crate::lib::pgwire::protocol::{BackendMessage, FormatCode};

use super::field_description::FieldDescription;

#[derive(Debug, Clone)]
pub struct RowDescription {
    pub fields: Vec<FieldDescription>,
    pub format_code: FormatCode,
}

impl BackendMessage for RowDescription {
    const TAG: u8 = b'T';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_i16(self.fields.len() as i16);
        for field in &self.fields {
            dst.put_slice(field.name.as_bytes());
            dst.put_u8(0);
            dst.put_i32(0); // table oid
            dst.put_i16(0); // column attr number
            dst.put_u32(field.data_type.into());
            dst.put_i16(field.data_type.size_bytes());
            dst.put_i32(-1); // data type modifier
            dst.put_i16(self.format_code as i16);
        }
    }
}
