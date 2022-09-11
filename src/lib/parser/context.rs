#[derive(Debug, Default, Clone)]
pub struct ParserContext {
    pub in_between_clause: bool,
    pub in_parentheses: bool,
    pub default_database: Option<String>,
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

    pub fn set_default_database(mut self, default_database: String) -> Self {
        self.default_database = Some(default_database);
        self
    }
}
