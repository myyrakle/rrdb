use crate::lib::ast::ddl::{CreateDatabaseQuery, DDLStatement, SQLStatement};
use crate::lib::ast::predule::{DMLStatement, OtherStatement, SQLExpression};
use crate::lib::errors::execute_error::ExecuteError;
use crate::lib::executor::predule::{ExecuteResult, GlobalConfig};
use crate::lib::logger::predule::Logger;
use crate::lib::optimizer::predule::Optimizer;
use crate::lib::utils::predule::set_system_env;
use path_absolutize::*;
use std::error::Error;
use std::path::PathBuf;

pub struct Executor {}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    // 기본 설정파일 세팅
    pub async fn reduce(&self, expression: SQLExpression) -> Result<SQLExpression, Box<dyn Error>> {
    }
}
