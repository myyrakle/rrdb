use crate::lib::ast::predule::SQLStatement;

pub struct Optimizer {}

impl Optimizer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn optimize(&self, _sql: &mut SQLStatement) -> () {
        // TODO: 최적화 작업
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}
