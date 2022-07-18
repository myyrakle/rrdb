use std::{convert::TryInto, error::Error};

use crate::lib::ast::predule::BinaryOperator;

#[derive(Clone, Debug, PartialEq)]
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
}

impl TryInto<BinaryOperator> for OperatorToken {
    type Error = Box<dyn Error>;

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
        }
    }
}
