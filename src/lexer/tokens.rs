use std::convert::TryInto;
use std::error::Error;

use super::predule::OperatorToken;
use crate::ast::predule::BinaryOperator;
use crate::errors::predule::IntoError;

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
    Nulls,
    First,
    Last,

    // DDL
    Create,
    Alter,
    Drop,
    Database,
    Table,
    Column,
    Comment,
    Primary,
    Foreign,
    Key,
    Add,
    If,
    Rename,
    To,
    Show,
    Databases,
    Tables,
    Use,
    Type,
    Default,
    Data,

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
    Is,

    // primary expression
    Identifier(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Null,

    Operator(OperatorToken),

    //functions
    Exists,

    // general syntax
    Comma,
    Period,
    SemiColon,
    LeftParentheses,
    RightParentheses,
    Backslash,

    // exception handling
    EOF,
    Error(String),
    UnknownCharacter(char),
}

impl Token {
    pub fn is_eof(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Token::EOF => true,
            _ => false,
        }
    }

    pub fn is_unary_operator(&self) -> bool {
        match self {
            Token::Operator(operator) => operator.is_unary_operator(),
            _ => false,
        }
    }

    // 복합 토큰으로 구성된 연산자일 수 있는 경우
    // IS NOT, NOT IN 등
    pub fn can_be_multi_token_operator(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Token::Not | Token::Is => true,
            _ => false,
        }
    }

    pub fn try_into_multi_token_operator(
        self,
        second_token: Self,
    ) -> Result<BinaryOperator, Box<dyn Error + Send>> {
        match self {
            Token::Not => match second_token {
                Token::Like => Ok(BinaryOperator::NotLike),
                Token::In => Ok(BinaryOperator::NotIn),
                _ => Err(IntoError::boxed("BinaryOperator Cast Error")),
            },
            Token::Is => match second_token {
                Token::Not => Ok(BinaryOperator::IsNot),
                _ => Ok(BinaryOperator::Is),
            },
            _ => Err(IntoError::boxed("BinaryOperator Cast Error")),
        }
    }

    pub fn is_expression(&self) -> bool {
        match self {
            Token::Identifier(_)
            | Token::Integer(_)
            | Token::Float(_)
            | Token::Boolean(_)
            | Token::String(_)
            | Token::Null
            | Token::LeftParentheses
            | Token::Not => true,
            Token::Operator(operator) => operator.is_unary_operator(),
            _ => false,
        }
    }
}

impl TryInto<BinaryOperator> for Token {
    type Error = Box<dyn Error + Send>;

    fn try_into(self) -> Result<BinaryOperator, Box<dyn Error + Send>> {
        match self {
            Token::Operator(operator) => operator.try_into(),
            Token::And => Ok(BinaryOperator::And),
            Token::Or => Ok(BinaryOperator::Or),
            Token::Like => Ok(BinaryOperator::Like),
            Token::In => Ok(BinaryOperator::In),
            Token::Is => Ok(BinaryOperator::Is),
            _ => Err(IntoError::boxed("BinaryOperator Cast Error")),
        }
    }
}
