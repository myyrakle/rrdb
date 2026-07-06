use crate::engine::DBEngine;
use crate::engine::ast::types::TableName;
use crate::engine::schema::row::TableDataRow;
use crate::engine::wal::endec::WALEncoder;
use crate::engine::wal::manager::WALManager;
use crate::engine::wal::types::{EntryType, WALEntry};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub(crate) async fn recover_from_wal<T>(
        &self,
        wal_manager: &mut WALManager<T>,
    ) -> errors::Result<()>
    where
        T: WALEncoder<WALEntry>,
    {
        self.replay_wal_entries(wal_manager.pending_entries())
            .await?;
        self.flush_row_buffers_durable().await?;
        wal_manager.flush().await
    }

    async fn replay_wal_entries(&self, entries: &[WALEntry]) -> errors::Result<()> {
        for entry in entries {
            if matches!(entry.entry_type, EntryType::Insert) {
                let Some(data) = entry.data.as_deref() else {
                    continue;
                };
                let (table_name, rows): (TableName, Vec<TableDataRow>) = bincode::deserialize(data)
                    .map_err(|error| ExecuteError::wrap(error.to_string()))?;
                self.append_table_rows(&table_name, &rows).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use tokio::io::AsyncWriteExt;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::types::TableName;
    use crate::engine::encoder::schema_encoder::StorageEncoder;
    use crate::engine::schema::row::{TableDataField, TableDataFieldType, TableDataRow};
    use crate::engine::wal::endec::WALEncoder;
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::engine::wal::types::{EntryType, WALEntry};

    async fn setup_base_path(test_name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base_path = PathBuf::from("target")
            .join("test_wal_recovery")
            .join(format!("{test_name}_{now}"));
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }
        tokio::fs::create_dir_all(&base_path).await.unwrap();
        base_path
    }

    async fn write_wal_entries(config: &LaunchConfig, entries: &[WALEntry]) {
        tokio::fs::create_dir_all(&config.wal_directory)
            .await
            .unwrap();
        let wal_path =
            Path::new(&config.wal_directory).join(format!("00000001.{}", config.wal_extension));
        let encoder = BincodeEncoder::new();
        let mut file = tokio::fs::File::create(&wal_path).await.unwrap();

        for entry in entries {
            let encoded_entry = encoder.encode(entry).unwrap();
            file.write_all(&u32::try_from(encoded_entry.len()).unwrap().to_le_bytes())
                .await
                .unwrap();
            file.write_all(&encoded_entry).await.unwrap();
        }

        file.sync_all().await.unwrap();
    }

    fn row(table_name: &TableName, value: i64) -> TableDataRow {
        TableDataRow {
            fields: vec![TableDataField {
                table_name: table_name.clone(),
                column_name: "id".to_string(),
                data: TableDataFieldType::Integer(value),
            }],
        }
    }

    #[tokio::test]
    async fn recover_from_wal_replays_insert_entries() {
        let base_path = setup_base_path("insert_entries").await;
        let config = LaunchConfig::default_for_base_path(&base_path);
        let table_name = TableName::new(Some("rrdb".to_string()), "users".to_string());
        let rows_path = PathBuf::from(&config.data_directory)
            .join("rrdb")
            .join("tables")
            .join("users")
            .join("rows");
        tokio::fs::create_dir_all(&rows_path).await.unwrap();

        let rows = vec![row(&table_name, 1), row(&table_name, 2)];
        let wal_payload = bincode::serialize(&(table_name.clone(), rows)).unwrap();
        let entry = WALEntry {
            entry_type: EntryType::Insert,
            data: Some(wal_payload),
            timestamp: 1,
            transaction_id: None,
            is_continuation: false,
        };
        write_wal_entries(&config, &[entry]).await;

        let mut wal_manager = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();
        let engine = DBEngine::new(config);

        engine.recover_from_wal(&mut wal_manager).await.unwrap();

        let segment_path = rows_path.join("00000001.rows");
        let content = tokio::fs::read(segment_path).await.unwrap();
        let decoder = StorageEncoder::new();
        let mut values = Vec::new();
        let mut offset = 0;

        while offset < content.len() {
            let frame_len = u32::from_le_bytes(
                content[offset..offset + size_of::<u32>()]
                    .try_into()
                    .unwrap(),
            ) as usize;
            offset += size_of::<u32>();
            let row: TableDataRow = decoder
                .decode(&content[offset..offset + frame_len])
                .unwrap();
            values.push(row.fields[0].data.clone());
            offset += frame_len;
        }

        assert_eq!(
            values,
            vec![
                TableDataFieldType::Integer(1),
                TableDataFieldType::Integer(2)
            ]
        );
        assert!(wal_manager.pending_entries().is_empty());
    }
}
