#[derive(Clone, Debug)]
pub enum BinaryOperator {
    Add, // A + B
    Sub, // A - B
    Mul, // A * B
    Div, // A / B
    And, // A AND B
    Or,  // A Or B
    Lt,  // A < B
    Gt,  // A > B
    Lte, // A <= B
    Gte, // A >= B
    Eq,  // A = B
    Neq, // A != B, A <> B
}

#[derive(Clone, Debug)]

pub enum UnaryOperator {
    Pos, // +A
    Neg, // -A
    Not, // Not A
}
