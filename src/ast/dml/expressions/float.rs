#[derive(Clone, Debug, PartialEq)]
pub struct FloatExpression {
    pub value: f64,
}

impl FloatExpression {
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::FloatExpression;

    #[test]
    fn test_new() {
        let float = FloatExpression::new(1.0);
        assert_eq!(float.value, 1.0);
    }
}
