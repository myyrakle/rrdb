use bytes::BytesMut;
use tokio_util::codec::Encoder;

use crate::lib::pgwire::protocol::{ConnectionCodec, FormatCode, ProtocolError, RowDescription};

use super::data_row_writer::DataRowWriter;

/// Supports batched rows for e.g. returning portal result sets.
///
/// NB: this struct only performs limited validation of column consistency across rows.
pub struct DataRowBatch {
    pub(crate) format_code: FormatCode,
    pub(crate) num_cols: usize,
    pub(crate) num_rows: usize,
    pub(crate) data: BytesMut,
    pub(crate) row: BytesMut,
}

impl DataRowBatch {
    /// Creates a new row batch using the given format code, requiring a certain number of columns per row.
    pub fn new(format_code: FormatCode, num_cols: usize) -> Self {
        Self {
            format_code,
            num_cols,
            num_rows: 0,
            data: BytesMut::new(),
            row: BytesMut::new(),
        }
    }

    /// Creates a [DataRowBatch] from the given [RowDescription].
    pub fn from_row_desc(desc: &RowDescription) -> Self {
        Self::new(desc.format_code, desc.fields.len())
    }

    /// Starts writing a new row.
    ///
    /// Returns a [DataRowWriter] that is responsible for the actual value encoding.
    pub fn create_row(&mut self) -> DataRowWriter {
        self.num_rows += 1;
        DataRowWriter::new(self)
    }

    /// Returns the number of rows currently written to this batch.
    pub fn num_rows(&self) -> usize {
        self.num_rows
    }
}

impl Encoder<DataRowBatch> for ConnectionCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: DataRowBatch, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend(item.data);
        Ok(())
    }
}
