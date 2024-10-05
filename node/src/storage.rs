use std::path::PathBuf;

use color_eyre::eyre::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zeropool_sharded_storage_common::config::StorageConfig;

pub struct Storage {
    path: PathBuf,
    config: StorageConfig,
}

impl Storage {
    pub fn new<P: Into<PathBuf>>(path: P, config: StorageConfig) -> Result<Self> {
        Ok(Self {
            path: path.into(),
            config,
        })
    }

    pub async fn write_sector(&self, data: Vec<u8>, index: usize) -> Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.path.join(index.to_string()))
            .await?;
        file.write_all(&data).await?;
        Ok(())
    }

    pub async fn read_sector(&self, index: usize) -> Result<Vec<u8>> {
        let mut file = tokio::fs::File::open(self.path.join(index.to_string())).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        Ok(data)
    }
}
