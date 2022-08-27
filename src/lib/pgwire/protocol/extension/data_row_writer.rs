use bytes::BufMut;
use chrono::{NaiveDate, NaiveDateTime};

use crate::lib::pgwire::protocol::FormatCode;

use super::DataRowBatch;

macro_rules! primitive_write {
    ($name: ident, $type: ident) => {
        #[allow(missing_docs)]
        pub fn $name(&mut self, val: $type) {
            match self.parent.format_code {
                FormatCode::Text => self.write_value(&val.to_string().into_bytes()),
                FormatCode::Binary => self.write_value(&val.to_be_bytes()),
            };
        }
    };
}

/// Temporarily leased from a [DataRowBatch] to encode a single row.
pub struct DataRowWriter<'a> {
    current_col: usize,
    parent: &'a mut DataRowBatch,
}

impl<'a> DataRowWriter<'a> {
    pub fn new(parent: &'a mut DataRowBatch) -> Self {
        parent.row.put_i16(parent.num_cols as i16);
        Self {
            current_col: 0,
            parent,
        }
    }

    fn write_value(&mut self, data: &[u8]) {
        self.current_col += 1;
        self.parent.row.put_i32(data.len() as i32);
        self.parent.row.put_slice(data);
    }

    /// Writes a null value for the next column.
    pub fn write_null(&mut self) {
        self.current_col += 1;
        self.parent.row.put_i32(-1);
    }

    /// Writes a string value for the next column.
    pub fn write_string(&mut self, val: &str) {
        self.write_value(val.as_bytes());
    }

    /// Writes a bool value for the next column.
    pub fn write_bool(&mut self, val: bool) {
        match self.parent.format_code {
            FormatCode::Text => self.write_value(if val { "t" } else { "f" }.as_bytes()),
            FormatCode::Binary => {
                self.current_col += 1;
                self.parent.row.put_u8(val as u8);
            }
        };
    }

    fn pg_date_epoch() -> NaiveDate {
        NaiveDate::from_ymd(2000, 1, 1)
    }

    /// Writes a date value for the next column.
    pub fn write_date(&mut self, val: NaiveDate) {
        match self.parent.format_code {
            FormatCode::Binary => {
                self.write_int4(val.signed_duration_since(Self::pg_date_epoch()).num_days() as i32)
            }
            FormatCode::Text => self.write_string(&val.to_string()),
        }
    }

    /// Writes a timestamp value for the next column.
    pub fn write_timestamp(&mut self, val: NaiveDateTime) {
        match self.parent.format_code {
            FormatCode::Binary => {
                self.write_int8(
                    val.signed_duration_since(Self::pg_date_epoch().and_hms(0, 0, 0))
                        .num_microseconds()
                        .unwrap(),
                );
            }
            FormatCode::Text => self.write_string(&val.to_string()),
        }
    }

    primitive_write!(write_int2, i16);
    primitive_write!(write_int4, i32);
    primitive_write!(write_int8, i64);
    primitive_write!(write_float4, f32);
    primitive_write!(write_float8, f64);
}

impl<'a> Drop for DataRowWriter<'a> {
    fn drop(&mut self) {
        assert_eq!(
            self.parent.num_cols, self.current_col,
            "dropped a row writer with an invalid number of columns"
        );

        self.parent.data.put_u8(b'D');
        self.parent.data.put_i32((self.parent.row.len() + 4) as i32);
        self.parent.data.extend(self.parent.row.split());
    }
}
