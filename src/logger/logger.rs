use colored::Colorize;

pub struct Logger {}

impl Logger {
    pub fn error(text: impl Into<String>) {
        println!("{}", format!("!![ERROR] {}", text.into()).red());
    }

    pub fn info(text: impl Into<String>) {
        println!("{}", format!("@@[INFO] {}", text.into()).green());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error() {
        Logger::error("This is an error message");
    }

    #[test]
    fn test_info() {
        Logger::info("This is an info message");
    }
}
