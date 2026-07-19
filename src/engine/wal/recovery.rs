use crate::engine::DBEngine;
use crate::engine::wal::endec::WALEncoder;
use crate::engine::wal::manager::WALManager;
use crate::engine::wal::types::WALEntry;
use crate::errors;

impl DBEngine {
    pub(crate) async fn recover_from_wal<T>(
        &self,
        wal_manager: &mut WALManager<T>,
    ) -> errors::Result<()>
    where
        T: WALEncoder<WALEntry>,
    {
        self.replay_wal(wal_manager.pending_entries()).await?;
        self.flush_row_buffers_durable().await?;
        wal_manager.flush().await
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::wal::endec::WALDecoder;
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::engine::wal::types::EntryType;

    use super::*;

    #[tokio::test]
    async fn failed_recovery_does_not_append_checkpoint() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base_path = PathBuf::from("target")
            .join("test_wal_recovery")
            .join(format!("failed_recovery_{suffix}"));
        let config = LaunchConfig::default_for_base_path(&base_path);
        tokio::fs::create_dir_all(&config.wal_directory)
            .await
            .unwrap();

        let mut wal_manager = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();
        wal_manager
            .append_record(EntryType::Insert, Some(vec![0xff]), None)
            .await
            .unwrap();
        wal_manager.sync().await.unwrap();

        let engine = DBEngine::new(config.clone());
        assert!(engine.recover_from_wal(&mut wal_manager).await.is_err());

        let wal_path =
            PathBuf::from(&config.wal_directory).join(format!("00000001.{}", config.wal_extension));
        let content = tokio::fs::read(wal_path).await.unwrap();
        let entries = BincodeDecoder::new().decode(&content).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(
            !entries
                .iter()
                .any(|entry| matches!(entry.entry_type, EntryType::Checkpoint))
        );
    }
}
