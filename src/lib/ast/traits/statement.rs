pub trait SQLStatement: std::fmt::Debug {}

pub trait DDLStatement: SQLStatement {}

pub trait DMLStatement: SQLStatement {}

pub trait DCLStatement: SQLStatement {}
