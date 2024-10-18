use std::{ops::Mul, path::PathBuf};

use color_eyre::eyre::Result;
use p3_field::{AbstractField, Field};
use p3_mersenne_31::Mersenne31;
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

    pub async fn write(&self, data: Vec<u8>, index: usize) -> Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(self.path.join(index.to_string()))
            .await?;
        file.write_all(&data).await?;
        Ok(())
    }

    pub async fn read(&self, index: usize) -> Result<Vec<Mersenne31>> {
        if !self.path.join(index.to_string()).exists() {
            return Ok(random_vec(self.config.sector_capacity(), index));
        }

        let mut file = tokio::fs::File::open(self.path.join(index.to_string())).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        let data = data
            .chunks(4)
            .map(|chunk| {
                Mersenne31::new(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            })
            .collect();

        Ok(data)
    }
}

fn random_vec(size: usize, index: usize) -> Vec<Mersenne31> {
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push(Mersenne31::new(3).exp_u64(index as u64 + i as u64));
    }
    data
}
