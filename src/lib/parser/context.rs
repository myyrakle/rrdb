#[derive(Debug, Default, Clone, Copy)]
pub struct ParserContext {
    pub in_between_clause: bool,
    pub in_parentheses: bool,
}

impl ParserContext {
    pub fn set_in_between_clause(mut self, in_between_clause: bool) -> Self {
        self.in_between_clause = in_between_clause;
        self
    }

    pub fn set_in_parentheses(mut self, in_parentheses: bool) -> Self {
        self.in_parentheses = in_parentheses;
        self
    }
}
