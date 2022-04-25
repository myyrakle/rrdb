// 2항연산자
#[derive(Clone, Debug, PartialEq)]
pub enum BinaryOperator {
    Add,  // A + B
    Sub,  // A - B
    Mul,  // A * B
    Div,  // A / B
    And,  // A AND B
    Or,   // A OR B
    Lt,   // A < B
    Gt,   // A > B
    Lte,  // A <= B
    Gte,  // A >= B
    Eq,   // A = B
    Neq,  // A != B, A <> B
    Like, // A LIKE B
}

// 단항연산자
#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOperator {
    Pos, // +A
    Neg, // -A
    Not, // Not A
}
