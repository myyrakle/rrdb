use crate::parser::predule::Parser;

impl Parser {
    #[allow(dead_code)]
    pub(crate) fn show_tokens(&self) {
        println!("{:?}", self);
    }
}
