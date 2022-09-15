use colored::Colorize;

pub struct Logger {}

impl Logger {
    pub fn error(text: impl Into<String>) {
        println!("{}", format!("!!ERROR: {}", text.into()).red());
    }
}
