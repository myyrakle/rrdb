#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntegerExpression {
    pub value: i64,
}

impl IntegerExpression {
    pub fn new(value: i64) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::IntegerExpression;

    #[test]
    fn test_new() {
        let integer = IntegerExpression::new(1);
        assert_eq!(integer.value, 1);
    }
}
