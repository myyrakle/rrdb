#[cfg(test)]

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

    let mut parser = Parser::new(text);
    let result = parser.parse();
    println!("{:?}", result);
}