use crate::engine::ast::dml::expressions::operators::{BinaryOperator, UnaryOperator};
use crate::errors::{ErrorKind, Errors};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperatorToken {
    Plus,     // +
    Minus,    // -
    Asterisk, // *
    Slash,    // /
    Lt,       // A < B
    Gt,       // A > B
    Lte,      // A <= B
    Gte,      // A >= B
    Eq,       // A = B
    Neq,      // A != B, A <> B
    Not,      // !A
}

impl OperatorToken {
    pub fn is_binary_operator(&self) -> bool {
        [
            Self::Plus,
            Self::Minus,
            Self::Asterisk,
            Self::Slash,
            Self::Lt,
            Self::Gt,
            Self::Lte,
            Self::Gte,
            Self::Eq,
            Self::Neq,
        ]
        .contains(self)
    }

    pub fn is_unary_operator(&self) -> bool {
        [Self::Plus, Self::Minus, Self::Not].contains(self)
    }
}

impl TryInto<BinaryOperator> for OperatorToken {
    type Error = Errors;

    fn try_into(self) -> Result<BinaryOperator, Self::Error> {
        match self {
            Self::Plus => Ok(BinaryOperator::Add),
            Self::Minus => Ok(BinaryOperator::Sub),
            Self::Asterisk => Ok(BinaryOperator::Mul),
            Self::Slash => Ok(BinaryOperator::Div),
            Self::Lt => Ok(BinaryOperator::Lt),
            Self::Gt => Ok(BinaryOperator::Gt),
            Self::Lte => Ok(BinaryOperator::Lte),
            Self::Gte => Ok(BinaryOperator::Gte),
            Self::Eq => Ok(BinaryOperator::Eq),
            Self::Neq => Ok(BinaryOperator::Neq),
            _ => Err(Errors::new(ErrorKind::IntoError(
                "BinaryOperator Cast Error".to_string(),
            ))),
        }
    }
}

impl TryInto<UnaryOperator> for OperatorToken {
    type Error = Errors;

    fn try_into(self) -> Result<UnaryOperator, Self::Error> {
        match self {
            Self::Plus => Ok(UnaryOperator::Pos),
            Self::Minus => Ok(UnaryOperator::Neg),
            Self::Not => Ok(UnaryOperator::Not),
            _ => Err(Errors::new(ErrorKind::IntoError(
                "UnaryOperator Cast Error".to_string(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_operator_token_is_binary_operator() {
        use super::OperatorToken;

        let test_cases = vec![
            (OperatorToken::Plus, true),
            (OperatorToken::Minus, true),
            (OperatorToken::Asterisk, true),
            (OperatorToken::Slash, true),
            (OperatorToken::Lt, true),
            (OperatorToken::Gt, true),
            (OperatorToken::Lte, true),
            (OperatorToken::Gte, true),
            (OperatorToken::Eq, true),
            (OperatorToken::Neq, true),
            (OperatorToken::Not, false),
        ];

        for (input, expected) in test_cases {
            let got = input.is_binary_operator();
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn test_operator_token_is_unary_operator() {
        use super::OperatorToken;

        let test_cases = vec![
            (OperatorToken::Plus, true),
            (OperatorToken::Minus, true),
            (OperatorToken::Asterisk, false),
            (OperatorToken::Slash, false),
            (OperatorToken::Lt, false),
            (OperatorToken::Gt, false),
            (OperatorToken::Lte, false),
            (OperatorToken::Gte, false),
            (OperatorToken::Eq, false),
            (OperatorToken::Neq, false),
            (OperatorToken::Not, true),
        ];

        for (input, expected) in test_cases {
            let got = input.is_unary_operator();
            assert_eq!(got, expected);
        }
    }

    #[test]
    fn test_operator_token_try_into_binary_operator() {
        use super::{BinaryOperator, OperatorToken};
        use std::convert::TryInto;

        struct TestCase {
            name: String,
            input: OperatorToken,
            want_error: bool,
            expected: BinaryOperator,
        }

        let test_cases = vec![
            TestCase {
                name: "연산자: +".to_owned(),
                input: OperatorToken::Plus,
                want_error: false,
                expected: BinaryOperator::Add,
            },
            TestCase {
                name: "연산자: -".to_owned(),
                input: OperatorToken::Minus,
                want_error: false,
                expected: BinaryOperator::Sub,
            },
            TestCase {
                name: "연산자: *".to_owned(),
                input: OperatorToken::Asterisk,
                want_error: false,
                expected: BinaryOperator::Mul,
            },
            TestCase {
                name: "연산자: /".to_owned(),
                input: OperatorToken::Slash,
                want_error: false,
                expected: BinaryOperator::Div,
            },
            TestCase {
                name: "연산자: <".to_owned(),
                input: OperatorToken::Lt,
                want_error: false,
                expected: BinaryOperator::Lt,
            },
            TestCase {
                name: "연산자: >".to_owned(),
                input: OperatorToken::Gt,
                want_error: false,
                expected: BinaryOperator::Gt,
            },
            TestCase {
                name: "연산자: <=".to_owned(),
                input: OperatorToken::Lte,
                want_error: false,
                expected: BinaryOperator::Lte,
            },
            TestCase {
                name: "연산자: >=".to_owned(),
                input: OperatorToken::Gte,
                want_error: false,
                expected: BinaryOperator::Gte,
            },
            TestCase {
                name: "연산자: =".to_owned(),
                input: OperatorToken::Eq,
                want_error: false,
                expected: BinaryOperator::Eq,
            },
            TestCase {
                name: "연산자: !=".to_owned(),
                input: OperatorToken::Neq,
                want_error: false,
                expected: BinaryOperator::Neq,
            },
            TestCase {
                name: "연산자: !".to_owned(),
                input: OperatorToken::Not,
                want_error: true,
                expected: BinaryOperator::Neq,
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

    #[test]
    fn test_operator_token_try_into_unary_operator() {
        use super::{OperatorToken, UnaryOperator};
        use std::convert::TryInto;

        struct TestCase {
            name: String,
            input: OperatorToken,
            want_error: bool,
            expected: UnaryOperator,
        }

        let test_cases = vec![
            TestCase {
                name: "연산자: +".to_owned(),
                input: OperatorToken::Plus,
                want_error: false,
                expected: UnaryOperator::Pos,
            },
            TestCase {
                name: "연산자: -".to_owned(),
                input: OperatorToken::Minus,
                want_error: false,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: *".to_owned(),
                input: OperatorToken::Asterisk,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: /".to_owned(),
                input: OperatorToken::Slash,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: <".to_owned(),
                input: OperatorToken::Lt,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: >".to_owned(),
                input: OperatorToken::Gt,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: <=".to_owned(),
                input: OperatorToken::Lte,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: >=".to_owned(),
                input: OperatorToken::Gte,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: =".to_owned(),
                input: OperatorToken::Eq,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: !=".to_owned(),
                input: OperatorToken::Neq,
                want_error: true,
                expected: UnaryOperator::Neg,
            },
            TestCase {
                name: "연산자: !".to_owned(),
                input: OperatorToken::Not,
                want_error: false,
                expected: UnaryOperator::Not,
            },
        ];

        for t in test_cases {
            let got = TryInto::<UnaryOperator>::try_into(t.input);

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
