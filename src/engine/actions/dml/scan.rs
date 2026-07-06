use std::collections::{HashMap, HashSet};
use std::io::ErrorKind as IOErrorKind;
use std::path::{Path, PathBuf};

use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use crate::engine::DBEngine;
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
                    .read_rows(segment_path, disk_rows)
            }
        };

        Ok(rows
            .into_iter()
            .enumerate()
            .map(|(row_index, row)| (RowLocation { row_index }, row))
            .collect())
    }

    pub(crate) async fn append_table_rows(
        &self,
        table_name: &TableName,
        rows: &[TableDataRow],
    ) -> errors::Result<()> {
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
    ) -> errors::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let _guard = self.row_storage_lock.lock().await;
        let segment_path = self.row_segment_path(table_name)?;
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

        Ok(())
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
                    .read_rows(segment_path.clone(), disk_rows)
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
                    .read_rows(segment_path.clone(), disk_rows)
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

    fn row_segment_path(&self, table_name: &TableName) -> errors::Result<PathBuf> {
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

    // TODO(#195): read only the frames covering the indexed rows instead of the
    // whole segment file once partial reads are supported.
    pub async fn index_scan(&self, _table_name: TableName) {}
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

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

        engine
            .append_table_rows(&table_name, &[row(1), row(2)])
            .await
            .unwrap();
        engine
            .update_table_rows(&table_name, [(1, row(20))].into())
            .await
            .unwrap();

        let scanned = engine.full_scan(table_name).await.unwrap();
        let segment_path = rows_path.join("00000001.rows");

        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(1));
        assert_eq!(scanned[1].1.fields[0].data, TableDataFieldType::Integer(20));
        assert!(!segment_path.exists());
    }

    #[tokio::test]
    async fn delete_table_rows_deletes_buffered_rows_without_flushing_segment_file() {
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
            .delete_table_rows(&table_name, [1].into())
            .await
            .unwrap();

        let scanned = engine.full_scan(table_name).await.unwrap();
        let segment_path = rows_path.join("00000001.rows");

        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(1));
        assert_eq!(scanned[1].1.fields[0].data, TableDataFieldType::Integer(3));
        assert!(!segment_path.exists());
    }

    #[tokio::test]
    async fn flush_rewrite_preserves_rows_appended_after_update() {
        let base_path = PathBuf::from(format!(
            "target/test_row_segments/rewrite_then_append_{}",
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
        engine
            .update_table_rows(&table_name, [(1, row(20))].into())
            .await
            .unwrap();
        engine
            .append_table_rows(&table_name, &[row(3)])
            .await
            .unwrap();
        engine.flush_row_buffers().await.unwrap();

        let scanned = engine.full_scan(table_name).await.unwrap();

        assert_eq!(scanned.len(), 3);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(1));
        assert_eq!(scanned[1].1.fields[0].data, TableDataFieldType::Integer(20));
        assert_eq!(scanned[2].1.fields[0].data, TableDataFieldType::Integer(3));
    }

    #[tokio::test]
    async fn append_table_rows_buffers_rows_until_flush() {
        let base_path = PathBuf::from(format!(
            "target/test_row_segments/buffered_segment_file_{}",
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

        let segment_path = rows_path.join("00000001.rows");
        assert!(!segment_path.exists());

        engine.flush_row_buffers().await.unwrap();

        let scanned = engine.full_scan(table_name).await.unwrap();
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[0].1.fields[0].data, TableDataFieldType::Integer(1));
        assert_eq!(scanned[1].1.fields[0].data, TableDataFieldType::Integer(2));
        assert!(segment_path.exists());
    }

    #[tokio::test]
    async fn append_table_rows_flushes_when_buffer_limit_is_reached() {
        let base_path = PathBuf::from(format!(
            "target/test_row_segments/buffer_pressure_flush_{}",
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
        let row = TableDataRow {
            fields: vec![TableDataField {
                table_name: table_name.clone(),
                column_name: "id".to_string(),
                data: TableDataFieldType::Integer(1),
            }],
        };

        engine
            .append_table_rows_with_buffer_limit(&table_name, &[row], 1)
            .await
            .unwrap();

        let segment_path = rows_path.join("00000001.rows");
        assert!(segment_path.exists());
        assert!(engine.row_buffer_pool.lock().await.is_dirty_empty());
        assert!(!engine.row_buffer_pool.lock().await.is_unsynced_empty());

        engine.flush_row_buffers_durable().await.unwrap();

        assert!(engine.row_buffer_pool.lock().await.is_unsynced_empty());
    }

    #[tokio::test]
    async fn append_table_rows_waits_for_row_storage_lock() {
        let base_path = PathBuf::from(format!(
            "target/test_row_segments/append_waits_for_storage_lock_{}",
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

        let engine = Arc::new(DBEngine::new(config));
        let row = TableDataRow {
            fields: vec![TableDataField {
                table_name: table_name.clone(),
                column_name: "id".to_string(),
                data: TableDataFieldType::Integer(1),
            }],
        };
        let _guard = engine.row_storage_lock.lock().await;
        let append_task = {
            let engine = engine.clone();
            tokio::spawn(async move { engine.append_table_rows(&table_name, &[row]).await })
        };

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        assert!(!append_task.is_finished());
    }
}
