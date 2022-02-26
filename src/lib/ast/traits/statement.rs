pub trait SQLStatement {}

pub trait DDLStatement: SQLStatement {}

pub trait DMLStatement: SQLStatement {}

pub trait DCLStatement: SQLStatement {}
