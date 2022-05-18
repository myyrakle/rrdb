#[derive(Clone, Debug, PartialEq)]
pub struct IntegerExpression {
    pub value: i64,
}

impl IntegerExpression {
    pub fn new(value: i64) -> Self {
        Self { value: value }
    }
}
