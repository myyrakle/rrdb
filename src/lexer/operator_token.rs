use crate::{
    ast::dml::expressions::operators::{BinaryOperator, UnaryOperator},
    errors::{predule::IntoError, RRDBError},
};

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
    type Error = RRDBError;

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
            _ => Err(IntoError::wrap("BinaryOperator Cast Error")),
        }
    }
}

impl TryInto<UnaryOperator> for OperatorToken {
    type Error = RRDBError;

    fn try_into(self) -> Result<UnaryOperator, Self::Error> {
        match self {
            Self::Plus => Ok(UnaryOperator::Pos),
            Self::Minus => Ok(UnaryOperator::Neg),
            Self::Not => Ok(UnaryOperator::Not),
            _ => Err(IntoError::wrap("UnaryOperator Cast Error")),
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

        let test_cases = vec![
            (OperatorToken::Plus, BinaryOperator::Add),
            (OperatorToken::Minus, BinaryOperator::Sub),
            (OperatorToken::Asterisk, BinaryOperator::Mul),
            (OperatorToken::Slash, BinaryOperator::Div),
            (OperatorToken::Lt, BinaryOperator::Lt),
            (OperatorToken::Gt, BinaryOperator::Gt),
            (OperatorToken::Lte, BinaryOperator::Lte),
            (OperatorToken::Gte, BinaryOperator::Gte),
            (OperatorToken::Eq, BinaryOperator::Eq),
            (OperatorToken::Neq, BinaryOperator::Neq),
        ];

        for (input, expected) in test_cases {
            let got: BinaryOperator = input.try_into().unwrap();
            assert_eq!(got, expected);
        }
    }
}
