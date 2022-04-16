use crate::lib::{DMLStatement, SQLStatement};

#[derive(Debug, Clone)]
pub struct UpdateQuery {}

impl DMLStatement for UpdateQuery {}

impl SQLStatement for UpdateQuery {}
