#[derive(Clone, Debug, PartialEq)]
pub struct FloatExpression {
    pub value: f64,
}

impl FloatExpression {
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}
