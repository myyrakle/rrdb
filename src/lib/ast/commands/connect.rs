#[derive(Clone, Debug, PartialEq)]
pub struct ConnectCommand {
    pub database_name: Option<String>,
}
