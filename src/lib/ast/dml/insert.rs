use crate::lib::{DMLStatement, SQLStatement, Table};

pub struct InsertQuery {
    pub into_table: Option<Table>,
}

impl DMLStatement for InsertQuery {}

impl SQLStatement for InsertQuery {}
