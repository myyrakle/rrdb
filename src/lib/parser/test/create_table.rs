use crate::lib::Parser;

#[test]
pub fn create_table() {
    let text = r#"
        CREATE TABLE person 
        (
            id INTEGER PRIMARY KEY,
            name TEXT,
            age INTEGER
        );
    "#.to_owned();

    let parser = Parser::new(text);
}