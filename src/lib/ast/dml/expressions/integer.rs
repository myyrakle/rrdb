#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegerExpression {
    pub value: i64,
}

impl IntegerExpression {
    pub fn new(value: i64) -> Self {
        Self { value }
    }
}
