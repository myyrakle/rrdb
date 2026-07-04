use std::collections::{HashMap, HashSet};
use std::io::ErrorKind as IOErrorKind;
use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

use crate::engine::DBEngine;
use crate::engine::ast::dml::plan::select::scan::IndexScanPlan;
use crate::engine::ast::types::TableName;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::row::TableDataRow;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

const ROW_SEGMENT_FILENAME: &str = "00000001.rows";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RowLocation {
    pub(crate) row_index: usize,
}

impl DBEngine {
    pub(crate) async fn full_scan(
        &self,
        table_name: TableName,
    ) -> errors::Result<Vec<(RowLocation, TableDataRow)>> {
        let segment_path = self.row_segment_path(&table_name)?;
        let rows = self.read_segment_rows(&segment_path).await?;

        Ok(rows
            .into_iter()
            .enumerate()
            .map(|(row_index, row)| (RowLocation { row_index }, row))
            .collect())
    }

    /// 행을 세그먼트 파일 끝에 추가하고 시작 row index를 반환합니다.
    /// 반환된 시작 인덱스는 인덱스 유지보수(key -> row index)에 사용됩니다.
    pub(crate) async fn append_table_rows(
        &self,
        table_name: &TableName,
        rows: &[TableDataRow],
    ) -> errors::Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }

        let _guard = self.row_storage_lock.lock().await;
        let segment_path = self.row_segment_path(table_name)?;

        // TODO(#217): 행 개수를 메타데이터로 유지해 O(n) 카운트를 제거
        let start_index = self.read_segment_rows(&segment_path).await?.len();

        let encoder = StorageEncoder::new();
        let mut frame = Vec::new();

        for row in rows {
            let encoded = encoder.encode(row);
            let frame_len = u32::try_from(encoded.len())
                .map_err(|_| ExecuteError::wrap("row frame is too large".to_string()))?;
            frame.extend_from_slice(&frame_len.to_le_bytes());
            frame.extend_from_slice(&encoded);
        }

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&segment_path)
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;

        file.write_all(&frame)
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;

        Ok(start_index)
    }

    pub(crate) async fn update_table_rows(
        &self,
        table_name: &TableName,
        replacements: HashMap<usize, TableDataRow>,
    ) -> errors::Result<()> {
        if replacements.is_empty() {
            return Ok(());
        }

        let _guard = self.row_storage_lock.lock().await;
        let segment_path = self.row_segment_path(table_name)?;
        let mut rows = self.read_segment_rows(&segment_path).await?;

        for (row_index, row) in replacements {
            let target = rows.get_mut(row_index).ok_or_else(|| {
                ExecuteError::wrap(format!("row index '{}' not found", row_index))
            })?;
            *target = row;
        }

        self.write_segment_rows(&segment_path, &rows).await
    }

    pub(crate) async fn delete_table_rows(
        &self,
        table_name: &TableName,
        row_indexes: HashSet<usize>,
    ) -> errors::Result<()> {
        if row_indexes.is_empty() {
            return Ok(());
        }

        let _guard = self.row_storage_lock.lock().await;
        let segment_path = self.row_segment_path(table_name)?;
        let rows = self.read_segment_rows(&segment_path).await?;
        let rows = rows
            .into_iter()
            .enumerate()
            .filter(|(row_index, _)| !row_indexes.contains(row_index))
            .map(|(_, row)| row)
            .collect::<Vec<_>>();

        self.write_segment_rows(&segment_path, &rows).await
    }

    async fn read_segment_rows(&self, segment_path: &Path) -> errors::Result<Vec<TableDataRow>> {
        let content = match tokio::fs::read(segment_path).await {
            Ok(content) => content,
            Err(error) if error.kind() == IOErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(ExecuteError::wrap(error.to_string())),
        };

        let encoder = StorageEncoder::new();
        let mut rows = Vec::new();
        let mut offset = 0;

        while offset < content.len() {
            if content.len() - offset < size_of::<u32>() {
                return Err(ExecuteError::wrap(
                    "truncated row segment frame header".to_string(),
                ));
            }

            let frame_len = u32::from_le_bytes(
                content[offset..offset + size_of::<u32>()]
                    .try_into()
                    .map_err(|error| ExecuteError::wrap(format!("{:?}", error)))?,
            ) as usize;
            offset += size_of::<u32>();

            if content.len() - offset < frame_len {
                return Err(ExecuteError::wrap(
                    "truncated row segment frame body".to_string(),
                ));
            }

            let row = encoder
                .decode::<TableDataRow>(&content[offset..offset + frame_len])
                .ok_or_else(|| ExecuteError::wrap("invalid row segment frame".to_string()))?;
            rows.push(row);
            offset += frame_len;
        }

        Ok(rows)
    }

    async fn write_segment_rows(
        &self,
        segment_path: &Path,
        rows: &[TableDataRow],
    ) -> errors::Result<()> {
        let encoder = StorageEncoder::new();
        let mut content = Vec::new();

        for row in rows {
            let encoded = encoder.encode(row);
            let frame_len = u32::try_from(encoded.len())
                .map_err(|_| ExecuteError::wrap("row frame is too large".to_string()))?;
            content.extend_from_slice(&frame_len.to_le_bytes());
            content.extend_from_slice(&encoded);
        }

        tokio::fs::write(segment_path, content)
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))
    }

    pub(crate) fn row_segment_path(&self, table_name: &TableName) -> errors::Result<PathBuf> {
        let database_name = table_name
            .database_name
            .as_ref()
            .ok_or_else(|| ExecuteError::wrap("database name is required".to_string()))?;

        Ok(self
            .get_data_directory()
            .join(database_name)
            .join("tables")
            .join(&table_name.table_name)
            .join("rows")
            .join(ROW_SEGMENT_FILENAME))
    }

    /// 인덱스 스캔: 인덱스에서 row index 목록을 조회한 뒤 해당 행만 반환합니다.
    pub(crate) async fn index_scan(
        &self,
        table_name: TableName,
        plan: &IndexScanPlan,
    ) -> errors::Result<Vec<(RowLocation, TableDataRow)>> {
        self.ensure_indices_loaded().await?;

        let row_paths: Vec<String> = match &plan.eq_key {
            Some(key) => self.index_manager.get(&plan.index_name, key).await?,
            None => self
                .index_manager
                .range(
                    &plan.index_name,
                    plan.start_key.as_deref(),
                    plan.end_key.as_deref(),
                )
                .await?
                .into_iter()
                .map(|entry| entry.row_path)
                .collect(),
        };

        if row_paths.is_empty() {
            return Ok(Vec::new());
        }

        // TODO(#195): 세그먼트 프레임 오프셋을 인덱스에 저장해 부분 읽기로 최적화
        let segment_path = self.row_segment_path(&table_name)?;
        let all_rows = self.read_segment_rows(&segment_path).await?;

        let mut result = Vec::with_capacity(row_paths.len());

        for row_path in row_paths {
            let row_index = row_path.parse::<usize>().map_err(|_| {
                ExecuteError::wrap(format!(
                    "index '{}' has invalid row path '{}'",
                    plan.index_name, row_path
                ))
            })?;

            match all_rows.get(row_index) {
                Some(row) => result.push((RowLocation { row_index }, row.clone())),
                None => {
                    return Err(ExecuteError::wrap(format!(
                        "index '{}' is out of sync with table data; drop and recreate the index",
                        plan.index_name
                    )));
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::types::TableName;
    use crate::engine::schema::row::{TableDataField, TableDataFieldType, TableDataRow};

    #[tokio::test]
    async fn append_table_rows_stores_rows_in_single_segment_file() {
        let base_path = PathBuf::from(format!(
            "target/test_row_segments/single_segment_file_{}",
            std::process::id()
        ));
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        let table_name = TableName::new(Some("rrdb".to_string()), "users".to_string());
        let rows_path = PathBuf::from(&config.data_directory)
            .join("rrdb")
            .join("tables")
            .join("users")
            .join("rows");
        tokio::fs::create_dir_all(&rows_path).await.unwrap();

        let engine = DBEngine::new(config);
        let row = |id| TableDataRow {
            fields: vec![TableDataField {
                table_name: table_name.clone(),
                column_name: "id".to_string(),
                data: TableDataFieldType::Integer(id),
            }],
        };

        engine
            .append_table_rows(&table_name, &[row(1), row(2)])
            .await
            .unwrap();

        let scanned = engine.full_scan(table_name).await.unwrap();
        let mut dir_entries = tokio::fs::read_dir(&rows_path).await.unwrap();
        let mut file_count = 0;
        while let Some(entry) = dir_entries.next_entry().await.unwrap() {
            if entry.file_type().await.unwrap().is_file() {
                file_count += 1;
            }
        }

        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(1));
        assert_eq!(scanned[1].1.fields[0].data, TableDataFieldType::Integer(2));
        assert_eq!(file_count, 1);
    }
}
