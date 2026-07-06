use std::collections::{HashMap, HashSet};
use std::io::ErrorKind as IOErrorKind;
use std::path::{Path, PathBuf};

use tokio::io::AsyncWriteExt;

use crate::engine::DBEngine;
use crate::engine::ast::types::TableName;
use crate::engine::encoder::schema_encoder::StorageEncoder;
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
        self.flush_row_buffers().await?;

        let segment_path = self.row_segment_path(&table_name)?;
        let rows = self.read_segment_rows(&segment_path).await?;

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

        let segment_path = self.row_segment_path(table_name)?;
        let frame = encode_row_frames(rows)?;

        let buffered_bytes = {
            let mut buffer = self.row_write_buffer.lock().await;
            buffer
                .entry(segment_path)
                .or_default()
                .extend_from_slice(&frame);
            buffer.values().map(Vec::len).sum::<usize>()
        };

        if buffered_bytes >= buffer_limit_bytes {
            self.flush_row_buffers().await?;
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
        self.flush_row_buffers_locked().await?;
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
        self.flush_row_buffers_locked().await?;
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
                .map_err(|error| {
                    ExecuteError::wrap(format!("invalid row segment frame: {}", error))
                })?;
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
        let content = encode_row_frames(rows)?;

        tokio::fs::write(segment_path, content)
            .await
            .map_err(|error| ExecuteError::wrap(error.to_string()))
    }

    pub(crate) async fn flush_row_buffers(&self) -> errors::Result<()> {
        let _guard = self.row_storage_lock.lock().await;
        self.flush_row_buffers_locked().await
    }

    async fn flush_row_buffers_locked(&self) -> errors::Result<()> {
        let pending = {
            let mut buffer = self.row_write_buffer.lock().await;
            if buffer.is_empty() {
                return Ok(());
            }

            std::mem::take(&mut *buffer)
        };

        for (segment_path, content) in pending {
            let mut file = match tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&segment_path)
                .await
            {
                Ok(file) => file,
                Err(error) => {
                    self.restore_row_buffer(segment_path, content).await;
                    return Err(ExecuteError::wrap(error.to_string()));
                }
            };

            if let Err(error) = file.write_all(&content).await {
                self.restore_row_buffer(segment_path, content).await;
                return Err(ExecuteError::wrap(error.to_string()));
            }
        }

        Ok(())
    }

    async fn restore_row_buffer(&self, segment_path: PathBuf, content: Vec<u8>) {
        let mut buffer = self.row_write_buffer.lock().await;
        let existing = buffer.entry(segment_path).or_default();
        let mut restored = content;
        restored.extend_from_slice(existing);
        *existing = restored;
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

fn encode_row_frames(rows: &[TableDataRow]) -> errors::Result<Vec<u8>> {
    let encoder = StorageEncoder::new();
    let mut content = Vec::new();

    for row in rows {
        let len_offset = content.len();
        content.extend_from_slice(&0u32.to_le_bytes());
        encoder
            .encode_into(&mut content, row)
            .map_err(|error| ExecuteError::wrap(error.to_string()))?;
        let frame_len = u32::try_from(content.len() - len_offset - size_of::<u32>())
            .map_err(|_| ExecuteError::wrap("row frame is too large".to_string()))?;
        content[len_offset..len_offset + size_of::<u32>()]
            .copy_from_slice(&frame_len.to_le_bytes());
    }

    Ok(content)
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
        assert!(engine.row_write_buffer.lock().await.is_empty());
    }
}
