use std::collections::{HashMap, HashSet};
use std::io::ErrorKind as IOErrorKind;
use std::path::{Path, PathBuf};

use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::engine::DBEngine;
use crate::engine::ast::dml::plan::select::scan::IndexScanPlan;
use crate::engine::ast::types::TableName;
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::row_buffer::{RowBufferWrite, encode_row_frames};
use crate::engine::schema::row::TableDataRow;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

const ROW_SEGMENT_FILENAME: &str = "00000001.rows";
const DEFAULT_ROW_WRITE_BUFFER_LIMIT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RowLocation {
    pub(crate) row_index: usize,
}

impl DBEngine {
    pub(crate) async fn full_scan(
        &self,
        table_name: TableName,
    ) -> errors::Result<Vec<(RowLocation, TableDataRow)>> {
        let _guard = self.row_storage_lock.lock().await;
        let segment_path = self.row_segment_path(&table_name)?;
        let cached_rows = { self.row_buffer_pool.lock().await.cached_rows(&segment_path) };
        let rows = match cached_rows {
            Some(rows) => rows,
            None => {
                let disk_rows = self.read_segment_rows(&segment_path).await?;
                self.row_buffer_pool
                    .lock()
                    .await
                    .read_rows(segment_path, || disk_rows)
            }
        };

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
        self.append_table_rows_with_buffer_limit(
            table_name,
            rows,
            DEFAULT_ROW_WRITE_BUFFER_LIMIT_BYTES,
        )
        .await
    }

    async fn append_table_rows_with_buffer_limit(
        &self,
        table_name: &TableName,
        rows: &[TableDataRow],
        buffer_limit_bytes: usize,
    ) -> errors::Result<usize> {
        if rows.is_empty() {
            return Ok(0);
        }

        let _guard = self.row_storage_lock.lock().await;
        let segment_path = self.row_segment_path(table_name)?;

        // 버퍼 풀의 현재 행 개수를 시작 인덱스로 사용.
        // persisted_rows가 적재되지 않았으면 먼저 디스크에서 읽어온다.
        let start_index = {
            let pool_guard = self.row_buffer_pool.lock().await;
            match pool_guard.cached_row_count(&segment_path) {
                Some(count) => count,
                None => {
                    drop(pool_guard);
                    let disk_rows = self.read_segment_rows(&segment_path).await?;
                    let count = disk_rows.len();
                    self.row_buffer_pool
                        .lock()
                        .await
                        .read_rows(segment_path.clone(), || disk_rows);
                    count
                }
            }
        };

        let frame = encode_row_frames(rows)?;

        let buffered_bytes = {
            self.row_buffer_pool
                .lock()
                .await
                .append_rows(segment_path, rows, frame)
        };

        if buffered_bytes >= buffer_limit_bytes {
            self.flush_row_buffers_locked(false).await?;
        }

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
        let cached_rows = { self.row_buffer_pool.lock().await.cached_rows(&segment_path) };
        let mut rows = match cached_rows {
            Some(rows) => rows,
            None => {
                let disk_rows = self.read_segment_rows(&segment_path).await?;
                self.row_buffer_pool
                    .lock()
                    .await
                    .read_rows(segment_path.clone(), || disk_rows)
            }
        };

        for (row_index, row) in replacements {
            let target = rows.get_mut(row_index).ok_or_else(|| {
                ExecuteError::wrap(format!("row index '{}' not found", row_index))
            })?;
            *target = row;
        }

        self.row_buffer_pool
            .lock()
            .await
            .replace_rows(segment_path, rows);

        Ok(())
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
        let cached_rows = { self.row_buffer_pool.lock().await.cached_rows(&segment_path) };
        let rows = match cached_rows {
            Some(rows) => rows,
            None => {
                let disk_rows = self.read_segment_rows(&segment_path).await?;
                self.row_buffer_pool
                    .lock()
                    .await
                    .read_rows(segment_path.clone(), || disk_rows)
            }
        };
        let rows = rows
            .into_iter()
            .enumerate()
            .filter(|(row_index, _)| !row_indexes.contains(row_index))
            .map(|(_, row)| row)
            .collect::<Vec<_>>();

        self.row_buffer_pool
            .lock()
            .await
            .replace_rows(segment_path, rows);

        Ok(())
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
                .map_err(|error| {
                    ExecuteError::wrap(format!("invalid row segment frame: {}", error))
                })?;
            rows.push(row);
            offset += frame_len;
        }

        Ok(rows)
    }

    #[cfg(test)]
    pub(crate) async fn flush_row_buffers(&self) -> errors::Result<()> {
        let _guard = self.row_storage_lock.lock().await;
        self.flush_row_buffers_locked(false).await
    }

    pub(crate) async fn flush_row_buffers_durable(&self) -> errors::Result<()> {
        let _guard = self.row_storage_lock.lock().await;
        self.flush_row_buffers_locked(true).await
    }

    async fn flush_row_buffers_locked(&self, durable: bool) -> errors::Result<()> {
        let pending = self.row_buffer_pool.lock().await.drain_writes()?;

        for write in pending {
            if let Err(error) = self.apply_row_buffer_write(&write, durable).await {
                self.row_buffer_pool.lock().await.restore_write(write);
                return Err(error);
            }
            self.row_buffer_pool
                .lock()
                .await
                .complete_write(write, durable);
        }

        if durable {
            self.sync_unsynced_row_segments().await?;
        }

        Ok(())
    }

    async fn apply_row_buffer_write(
        &self,
        write: &RowBufferWrite,
        durable: bool,
    ) -> errors::Result<()> {
        let existing_len = if write.replace_existing {
            0
        } else {
            match tokio::fs::metadata(&write.segment_path).await {
                Ok(metadata) => metadata.len(),
                Err(error) if error.kind() == IOErrorKind::NotFound => 0,
                Err(error) => return Err(ExecuteError::wrap(error.to_string())),
            }
        };

        let mut file = match tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&write.segment_path)
            .await
        {
            Ok(file) => file,
            Err(error) => return Err(ExecuteError::wrap(error.to_string())),
        };

        file.set_len(existing_len)
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;
        file.seek(std::io::SeekFrom::End(0))
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;
        file.write_all(&write.content)
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;

        if durable {
            file.sync_data()
                .await
                .map_err(|error| ExecuteError::wrap(error.to_string()))?;
        }

        Ok(())
    }

    async fn sync_unsynced_row_segments(&self) -> errors::Result<()> {
        let mut unsynced_segments = self.row_buffer_pool.lock().await.drain_unsynced_segments();

        while let Some(segment_path) = unsynced_segments.pop() {
            if let Err(error) = self.sync_row_segment(&segment_path).await {
                let mut row_buffer_pool = self.row_buffer_pool.lock().await;
                row_buffer_pool.mark_unsynced_segment(segment_path);
                for remaining_segment_path in unsynced_segments {
                    row_buffer_pool.mark_unsynced_segment(remaining_segment_path);
                }
                return Err(error);
            }
        }

        Ok(())
    }

    async fn sync_row_segment(&self, segment_path: &Path) -> errors::Result<()> {
        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(segment_path)
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;

        file.sync_data()
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
        let _guard = self.row_storage_lock.lock().await;
        let segment_path = self.row_segment_path(&table_name)?;
        let cached_rows = { self.row_buffer_pool.lock().await.cached_rows(&segment_path) };
        let all_rows = match cached_rows {
            Some(rows) => rows,
            None => {
                let disk_rows = self.read_segment_rows(&segment_path).await?;
                self.row_buffer_pool
                    .lock()
                    .await
                    .read_rows(segment_path, || disk_rows)
            }
        };

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
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::types::TableName;
    use crate::engine::schema::row::{TableDataField, TableDataFieldType, TableDataRow};

    #[tokio::test]
    async fn full_scan_reads_buffered_rows_without_flushing_segment_file() {
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
        let segment_path = rows_path.join("00000001.rows");

        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(1));
        assert_eq!(scanned[1].1.fields[0].data, TableDataFieldType::Integer(2));
        assert!(!segment_path.exists());
    }

    #[tokio::test]
    async fn update_table_rows_updates_buffered_rows_without_flushing_segment_file() {
        let base_path = PathBuf::from(format!(
            "target/test_row_segments/update_buffered_rows_{}",
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
        let row2 = |id| TableDataRow {
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

        let table_name_clone = table_name.clone();
        engine
            .update_table_rows(
                &table_name_clone,
                HashMap::from([(0, TableDataRow {
                    fields: vec![TableDataField {
                        table_name: table_name.clone(),
                        column_name: "id".to_string(),
                        data: TableDataFieldType::Integer(10),
                    }],
                })]),
            )
            .await
            .unwrap();

        let scanned = engine.full_scan(table_name).await.unwrap();
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(10));
        assert_eq!(scanned[1].1.fields[0].data, TableDataFieldType::Integer(2));
    }

    #[tokio::test]
    async fn delete_table_rows_removes_buffered_rows_without_flushing_segment_file() {
        let base_path = PathBuf::from(format!(
            "target/test_row_segments/delete_buffered_rows_{}",
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
            .append_table_rows(&table_name, &[row(1), row(2), row(3)])
            .await
            .unwrap();

        engine
            .delete_table_rows(&table_name, HashSet::from([0, 2]))
            .await
            .unwrap();

        let scanned = engine.full_scan(table_name).await.unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(2));
    }
}
