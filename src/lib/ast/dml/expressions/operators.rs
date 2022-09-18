use serde::{Deserialize, Serialize};

// 2항연산자
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,     // A + B
    Sub,     // A - B
    Mul,     // A * B
    Div,     // A / B
    And,     // A AND B
    Or,      // A OR B
    Lt,      // A < B
    Gt,      // A > B
    Lte,     // A <= B
    Gte,     // A >= B
    Eq,      // A = B
    Neq,     // A != B, A <> B
    Like,    // A LIKE B
    NotLike, // A NOT LIKE B
    In,      // A In B
    NotIn,   // A Not In B
    Is,      // A Is B
    IsNot,   // A Is Not B
}

// 단항연산자
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Pos, // +A
    Neg, // -A
    Not, // Not A
}

impl BinaryOperator {
    // 2항연산자 우선순위 획득
    pub fn get_precedence(&self) -> i32 {
        match self {
            BinaryOperator::Add => 10,
            BinaryOperator::Sub => 10,
            BinaryOperator::Mul => 40,
            BinaryOperator::Div => 40,
            BinaryOperator::And => 10,
            BinaryOperator::Or => 10,
            BinaryOperator::Lt => 10,
            BinaryOperator::Gt => 10,
            BinaryOperator::Lte => 10,
            BinaryOperator::Gte => 10,
            BinaryOperator::Eq => 10,
            BinaryOperator::Neq => 10,
            BinaryOperator::Like => 10,
            BinaryOperator::NotLike => 10,
            BinaryOperator::In => 10,
            BinaryOperator::NotIn => 10,
            BinaryOperator::Is => 10,
            BinaryOperator::IsNot => 10,
        }
    }
}
