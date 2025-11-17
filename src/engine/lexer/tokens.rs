use super::predule::OperatorToken;
use crate::engine::ast::dml::expressions::operators::BinaryOperator;
use crate::errors::RRDBError;
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

    // TCL
    Begin,
    Transaction,
    Commit,
    Rollback,

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
    ) -> Result<BinaryOperator, RRDBError> {
        match self {
            Token::Not => match second_token {
                Token::Like => Ok(BinaryOperator::NotLike),
                Token::In => Ok(BinaryOperator::NotIn),
                _ => Err(IntoError::wrap("BinaryOperator Cast Error")),
            },
            Token::Is => match second_token {
                Token::Not => Ok(BinaryOperator::IsNot),
                _ => Ok(BinaryOperator::Is),
            },
            _ => Err(IntoError::wrap("BinaryOperator Cast Error")),
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
    type Error = RRDBError;

    fn try_into(self) -> Result<BinaryOperator, RRDBError> {
        match self {
            Token::Operator(operator) => operator.try_into(),
            Token::And => Ok(BinaryOperator::And),
            Token::Or => Ok(BinaryOperator::Or),
            Token::Like => Ok(BinaryOperator::Like),
            Token::In => Ok(BinaryOperator::In),
            Token::Is => Ok(BinaryOperator::Is),
            _ => Err(IntoError::wrap("BinaryOperator Cast Error")),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::{
        ast::dml::expressions::operators::BinaryOperator,
        lexer::{predule::OperatorToken, tokens::Token},
    };

    #[test]
    fn test_token_is_unary_operator() {
        let test_cases = vec![
            (Token::Operator(OperatorToken::Plus), true),
            (Token::Operator(OperatorToken::Minus), true),
            (Token::Operator(OperatorToken::Not), true),
            (Token::And, false),
            (Token::Or, false),
            (Token::Like, false),
            (Token::In, false),
            (Token::Is, false),
            (Token::Identifier("test".to_owned()), false),
            (Token::Integer(1), false),
            (Token::Float(1.0), false),
            (Token::Boolean(true), false),
            (Token::String("test".to_owned()), false),
            (Token::Null, false),
            (Token::LeftParentheses, false),
        ];

        for (token, want) in test_cases {
            assert_eq!(token.is_unary_operator(), want);
        }
    }

    #[test]
    fn test_token_try_into_multi_token_operator() {
        struct TestCase {
            name: String,
            first: Token,
            second: Token,
            expected: BinaryOperator,
            want_error: bool,
        }

        let test_cases = vec![
            TestCase {
                name: "NOT LIKE".into(),
                first: Token::Not,
                second: Token::Like,
                expected: BinaryOperator::NotLike,
                want_error: false,
            },
            TestCase {
                name: "NOT IN".into(),
                first: Token::Not,
                second: Token::In,
                expected: BinaryOperator::NotIn,
                want_error: false,
            },
            TestCase {
                name: "IS NOT".into(),
                first: Token::Is,
                second: Token::Not,
                expected: BinaryOperator::IsNot,
                want_error: false,
            },
            TestCase {
                name: "IS".into(),
                first: Token::Is,
                second: Token::Is,
                expected: BinaryOperator::Is,
                want_error: false,
            },
            TestCase {
                name: "NOT AND".into(),
                first: Token::Not,
                second: Token::And,
                expected: BinaryOperator::Is,
                want_error: true,
            },
            TestCase {
                name: "IS LIKE".into(),
                first: Token::Is,
                second: Token::Like,
                expected: BinaryOperator::Is,
                want_error: false,
            },
            TestCase {
                name: "Error".into(),
                first: Token::Select,
                second: Token::Like,
                expected: BinaryOperator::Is,
                want_error: true,
            },
        ];

        for t in test_cases {
            let got = t.first.try_into_multi_token_operator(t.second);

            assert_eq!(
                got.is_err(),
                t.want_error,
                "{}: want_error: {}, error: {:?}",
                t.name,
                t.want_error,
                got.err()
            );

            if let Ok(tokens) = got {
                assert_eq!(tokens, t.expected, "TC: {}", t.name);
            }
        }
    }

    #[test]
    fn test_token_is_expression() {
        struct TestCase {
            name: String,
            input: Token,
            expected: bool,
        }

        let test_cases = vec![
            TestCase {
                name: "Identifier".into(),
                input: Token::Identifier("test".into()),
                expected: true,
            },
            TestCase {
                name: "Integer".into(),
                input: Token::Integer(1),
                expected: true,
            },
            TestCase {
                name: "Float".into(),
                input: Token::Float(1.0),
                expected: true,
            },
            TestCase {
                name: "Boolean".into(),
                input: Token::Boolean(true),
                expected: true,
            },
            TestCase {
                name: "String".into(),
                input: Token::String("test".into()),
                expected: true,
            },
            TestCase {
                name: "Null".into(),
                input: Token::Null,
                expected: true,
            },
            TestCase {
                name: "LeftParentheses".into(),
                input: Token::LeftParentheses,
                expected: true,
            },
            TestCase {
                name: "Not".into(),
                input: Token::Not,
                expected: true,
            },
            TestCase {
                name: "Operator".into(),
                input: Token::Operator(OperatorToken::Plus),
                expected: true,
            },
            TestCase {
                name: "And".into(),
                input: Token::And,
                expected: false,
            },
            TestCase {
                name: "Or".into(),
                input: Token::Or,
                expected: false,
            },
            TestCase {
                name: "Like".into(),
                input: Token::Like,
                expected: false,
            },
            TestCase {
                name: "In".into(),
                input: Token::In,
                expected: false,
            },
            TestCase {
                name: "Is".into(),
                input: Token::Is,
                expected: false,
            },
            TestCase {
                name: "Comma".into(),
                input: Token::Comma,
                expected: false,
            },
            TestCase {
                name: "Period".into(),
                input: Token::Period,
                expected: false,
            },
            TestCase {
                name: "SemiColon".into(),
                input: Token::SemiColon,
                expected: false,
            },
            TestCase {
                name: "RightParentheses".into(),
                input: Token::RightParentheses,
                expected: false,
            },
            TestCase {
                name: "EOF".into(),
                input: Token::EOF,
                expected: false,
            },
        ];

        for t in test_cases {
            let got = t.input.is_expression();
            assert_eq!(got, t.expected, "TC: {}", t.name);
        }
    }

    #[test]
    fn test_token_try_into_binary_operator() {
        struct TestCase {
            name: String,
            input: Token,
            expected: BinaryOperator,
            want_error: bool,
        }

        let test_cases = vec![
            TestCase {
                name: "AND".into(),
                input: Token::And,
                expected: BinaryOperator::And,
                want_error: false,
            },
            TestCase {
                name: "OR".into(),
                input: Token::Or,
                expected: BinaryOperator::Or,
                want_error: false,
            },
            TestCase {
                name: "LIKE".into(),
                input: Token::Like,
                expected: BinaryOperator::Like,
                want_error: false,
            },
            TestCase {
                name: "IN".into(),
                input: Token::In,
                expected: BinaryOperator::In,
                want_error: false,
            },
            TestCase {
                name: "IS".into(),
                input: Token::Is,
                expected: BinaryOperator::Is,
                want_error: false,
            },
            TestCase {
                name: "Identifier".into(),
                input: Token::Identifier("test".into()),
                expected: BinaryOperator::Is,
                want_error: true,
            },
        ];

        for t in test_cases {
            let got = TryInto::<BinaryOperator>::try_into(t.input);

            assert_eq!(
                got.is_err(),
                t.want_error,
                "{}: want_error: {}, error: {:?}",
                t.name,
                t.want_error,
                got.err()
            );

            if let Ok(tokens) = got {
                assert_eq!(tokens, t.expected, "TC: {}", t.name);
            }
        }
    }
}
