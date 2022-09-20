use std::error::Error;

use crate::lib::ast::predule::{SelectQuery, TableName};
use crate::lib::executor::predule::{ExecuteResult, Executor};
use crate::lib::optimizer::predule::Optimizer;

impl Executor {
    pub async fn select(&self, query: SelectQuery) -> Result<ExecuteResult, Box<dyn Error>> {
        // 최적화 작업
        let optimizer = Optimizer::new();
        let plan = optimizer.optimize(query).await?;

        todo!();
    }

    pub async fn full_scan(&self, _table_name: TableName) {}

    pub async fn filter(&self) {}

    pub async fn index_scan(&self, _table_name: TableName) {}

    pub async fn order_by(&self) {}

    pub async fn inner_join(&self) {}
}
