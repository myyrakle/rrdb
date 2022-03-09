#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // DCL
    Grant,
    Revoke,

    // DML
    Select,
    From,
    Where,
    As,
    Order,
    By,
    Asc,
    Desc,
    Group,
    Having,
    Limit,
    Offset,
    Insert,
    Into,
    Update,
    Set,
    Delete,
    Join,
    Inner,
    Left,
    Right,
    Full,
    Outer,
    On,

    // DDL
    Create,
    Alter,
    Drop,
    Table,
    Column,
    Comment,
    Key,
    Add,

    // ETC
    Analyze,

    // EXPRESSION
    And,
    Or,
    Not,
    Between,
    Like,
    In,

    // primary
    Identifier(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),

    // exception handling
    EOF,
    Error(String),
    UnknownCharacter(char),
}
