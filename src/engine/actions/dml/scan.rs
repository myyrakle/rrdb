use crate::engine::DBEngine;
use crate::engine::ast::types::TableName;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::row::TableDataRow;
use crate::engine::storage::{RowId, TableHeap};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn full_scan(
        &self,
        table_name: TableName,
    ) -> errors::Result<Vec<(RowId, TableDataRow)>> {
        let encoder = StorageEncoder::new();

        let mut heaps = self.table_heaps.write().await;
        let heap = heaps.entry(table_name).or_insert_with(TableHeap::new);
        let rows = heap
            .scan()
            .map_err(|error| ExecuteError::wrap(format!("{:?}", error)))?;
        drop(heaps);

        let mut decoded = Vec::with_capacity(rows.len());
        for (row_id, data) in rows {
            let row = encoder
                .decode::<TableDataRow>(data.as_slice())
                .ok_or_else(|| ExecuteError::wrap("full scan failed".to_string()))?;
            decoded.push((row_id, row));
        }

        Ok(decoded)
    }

    pub async fn index_scan(&self, _table_name: TableName) {}
}
