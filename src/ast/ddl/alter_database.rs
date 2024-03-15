use crate::ast::predule::{DDLStatement, SQLStatement};

/*
ALTER DATABASE database_name
{
    RENAME TO new_database_name
};
*/
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlterDatabaseQuery {
    pub database_name: Option<String>,
    pub action: Option<AlterDatabaseAction>,
}

impl AlterDatabaseQuery {
    pub fn builder() -> Self {
        AlterDatabaseQuery {
            database_name: None,
            action: None,
        }
    }

    pub fn set_name(mut self, name: String) -> Self {
        self.database_name = Some(name);
        self
    }

    pub fn set_action(mut self, action: AlterDatabaseAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn build(self) -> SQLStatement {
        SQLStatement::DDL(DDLStatement::AlterDatabase(self))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlterDatabaseAction {
    RenameTo(AlterDatabaseRenameTo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlterDatabaseRenameTo {
    pub name: String,
}
