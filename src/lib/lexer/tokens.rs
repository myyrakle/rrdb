#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // DCL
    // Grant,
    // Revoke,

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
    Values,
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
    Primary,
    Foreign,
    Key,
    Add,
    If,

    // ETC
    // Analyze,
    CodeComment(String),

    // EXPRESSION
    And,
    Or,
    Not,
    Between,
    Like,
    In,

    // primary expression
    Identifier(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),

    Operator(String),

    //functions
    Exists,

    // general syntax
    Comma,
    Period,
    SemiColon,
    LeftParentheses,
    RightParentheses,

    // exception handling
    EOF,
    Error(String),
    UnknownCharacter(char),
}

impl Token {
    pub fn is_eof(&self) -> bool {
        match self {
            Token::EOF => true,
            _ => false,
        }
    }
}
