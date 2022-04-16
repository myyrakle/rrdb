use crate::lib::{DMLStatement, SQLStatement};

#[derive(Debug, Clone)]
pub struct DeleteQuery {}

impl DMLStatement for DeleteQuery {}

impl SQLStatement for DeleteQuery {}
