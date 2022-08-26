use super::BindFormat;

#[derive(Debug)]
pub struct Bind {
    pub portal: String,
    pub prepared_statement_name: String,
    pub result_format: BindFormat,
}
