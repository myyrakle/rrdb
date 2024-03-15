use crate::pgwire::protocol::FormatCode;

#[derive(Debug)]
pub enum BindFormat {
    All(FormatCode),
    PerColumn(Vec<FormatCode>),
}
