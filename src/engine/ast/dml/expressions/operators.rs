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
    // 표준 SQL 우선순위: OR < AND < 비교 연산자 < 덧셈/뺄셈 < 곱셈/나눗셈
    pub fn get_precedence(&self) -> i32 {
        match self {
            BinaryOperator::Or => 1,
            BinaryOperator::And => 2,
            BinaryOperator::Lt => 5,
            BinaryOperator::Gt => 5,
            BinaryOperator::Lte => 5,
            BinaryOperator::Gte => 5,
            BinaryOperator::Eq => 5,
            BinaryOperator::Neq => 5,
            BinaryOperator::Like => 5,
            BinaryOperator::NotLike => 5,
            BinaryOperator::In => 5,
            BinaryOperator::NotIn => 5,
            BinaryOperator::Is => 5,
            BinaryOperator::IsNot => 5,
            BinaryOperator::Add => 10,
            BinaryOperator::Sub => 10,
            BinaryOperator::Mul => 40,
            BinaryOperator::Div => 40,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BinaryOperator;

    #[test]
    fn test_get_precedence() {
        assert_eq!(BinaryOperator::Add.get_precedence(), 10);
        assert_eq!(BinaryOperator::Sub.get_precedence(), 10);
        assert_eq!(BinaryOperator::Mul.get_precedence(), 40);
        assert_eq!(BinaryOperator::Div.get_precedence(), 40);
        assert_eq!(BinaryOperator::And.get_precedence(), 2);
        assert_eq!(BinaryOperator::Or.get_precedence(), 1);
        assert_eq!(BinaryOperator::Lt.get_precedence(), 5);
        assert_eq!(BinaryOperator::Gt.get_precedence(), 5);
        assert_eq!(BinaryOperator::Lte.get_precedence(), 5);
        assert_eq!(BinaryOperator::Gte.get_precedence(), 5);
        assert_eq!(BinaryOperator::Eq.get_precedence(), 5);
        assert_eq!(BinaryOperator::Neq.get_precedence(), 5);
        assert_eq!(BinaryOperator::Like.get_precedence(), 5);
        assert_eq!(BinaryOperator::NotLike.get_precedence(), 5);
        assert_eq!(BinaryOperator::In.get_precedence(), 5);
        assert_eq!(BinaryOperator::NotIn.get_precedence(), 5);
        assert_eq!(BinaryOperator::Is.get_precedence(), 5);
        assert_eq!(BinaryOperator::IsNot.get_precedence(), 5);
    }
}
