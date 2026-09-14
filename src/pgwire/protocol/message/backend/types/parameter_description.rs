use bytes::{BufMut, BytesMut};

use crate::pgwire::protocol::DataTypeOid;
use crate::pgwire::protocol::backend::BackendMessage;

#[derive(Debug, Default)]
pub struct ParameterDescription {
    pub parameter_types: Vec<DataTypeOid>,
}

impl BackendMessage for ParameterDescription {
    const TAG: u8 = b't';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_i16(self.parameter_types.len() as i16);
        for oid in &self.parameter_types {
            dst.put_u32((*oid).into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_length_and_each_parameter_oid() {
        let desc = ParameterDescription {
            parameter_types: vec![DataTypeOid::Int4, DataTypeOid::Text],
        };
        let mut buf = BytesMut::new();
        desc.encode(&mut buf);

        // i16 count | u32 oid | u32 oid
        // Int4 = 23, Text = 25
        assert_eq!(
            &buf[..],
            &[0, 2, 0, 0, 0, 23, 0, 0, 0, 25][..],
        );
    }

    #[test]
    fn encodes_empty_parameter_list_as_zero_count() {
        let desc = ParameterDescription::default();
        let mut buf = BytesMut::new();
        desc.encode(&mut buf);
        assert_eq!(&buf[..], &[0, 0][..]);
    }
}
