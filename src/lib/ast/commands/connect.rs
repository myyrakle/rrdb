#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectCommand {
    pub database_name: Option<String>,
}
