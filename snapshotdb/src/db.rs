//! SnapshotDB: A Sharded Storage System with Snapshot Management
//! 
//! This module implements a sharded storage system that provides efficient cluster-based data storage
//! with snapshot management capabilities. The system allows for concurrent read/write operations
//! while maintaining data consistency through snapshots.
//!
//! # Key Features
//! - Cluster-based data storage with configurable cluster sizes
//! - Snapshot management for data versioning
//! - Concurrent read/write operations using tokio async runtime
//! - Efficient space allocation and deallocation
//! - Persistent storage with sled backend
//!
//! # Architecture
//! The system consists of several key components:
//! - SnapshotDb: Main database structure managing storage and snapshots
//! - OffsetTable: Manages data location mapping for different snapshots
//! - Allocator: Handles space allocation and deallocation
//! - SledWrapper: Provides persistent storage capabilities

use hashbrown::HashMap;
use tokio::sync::{RwLock, Mutex};
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt, AsyncReadExt, SeekFrom};
use std::io::Result;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::fs::{OpenOptions, File as StdFile};
use std::sync::Arc;

use crate::allocator::{Allocator, FREE_SLOTS_MIN_RESERVE};
use crate::sledwrapper::{OffsetTableEntry, SledWrapper, SledKey};
use crate::utils::{custom_sync_range, mutex_vec_values, to_mutex_vec};

/// Configuration for the SnapshotDb instance
#[derive(Debug, Clone, Copy)]
pub struct SnapshotDbConfig {
    /// Number of clusters in the storage system
    pub num_clusters: usize,
    /// Size of each cluster in bytes
    pub cluster_size: usize,
}

/// Main database structure managing storage and snapshots
pub struct SnapshotDb {
    /// Persistent storage backend
    db: SledWrapper,
    /// Database configuration
    config: SnapshotDbConfig,
    /// Table mapping snapshots to their data locations
    offset_table: RwLock<OffsetTable>,
    /// Space allocation manager
    allocator: Allocator,
    /// Total number of available slots
    num_slots: AtomicUsize,
    /// File handle for data storage
    storage: Arc<StdFile>
}

/// Manages mapping between snapshots and their data locations
pub struct OffsetTable {
    /// ID of the earliest active snapshot
    snapshot_start: usize,
    /// ID of the latest pending snapshot
    snapshot_pending: usize,
    /// Maps snapshot IDs to cluster offset entries
    inner: HashMap<usize, Vec<Mutex<OffsetTableEntry>>>
}

impl SnapshotDb {
    /// Creates a new SnapshotDb instance with the specified path and configuration
    ///
    /// # Arguments
    /// * `path` - Base path for database files
    /// * `config` - Database configuration
    ///
    /// # Returns
    /// * `Result<Self>` - New SnapshotDb instance or IO error
    pub async fn new(path: impl AsRef<Path>, config: SnapshotDbConfig) -> Result<Self> {
        let sled_path = path.as_ref().join("sled");
        let storage_path = path.as_ref().join("storage");

        let db = SledWrapper::new(sled::open(sled_path)?);
        let storage = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(storage_path)?;


        if db.is_empty()? {
            init_db(&db, &storage, &config).await?;
        }

        let num_slots = db.get_num_slots()?;
        let offset_table_vec = init_offset_table(&db, &config)?;

        let mut link_counter = vec![0; num_slots];

        for slot in offset_table_vec.iter() {
            for entry in slot.iter() {
                if let Some(entry) = entry {
                    link_counter[entry.offset as usize] += 1;
                }
            }
        }

        let allocator = Allocator::from_link_counter(link_counter);

        let mut offset_table = HashMap::new();

        for i in 0..offset_table_vec.len() {
            let slot = offset_table_vec[i].clone().into_iter().map(|entry| Mutex::new(entry.unwrap())).collect();
            offset_table.insert(i, slot);
        }

        let snapshot_start = db.get_snapshot_start()?;
        let snapshot_pending = db.get_snapshot_pending()?;

        Ok(Self { db, config, offset_table: RwLock::new(OffsetTable { snapshot_start, snapshot_pending, inner: offset_table }), allocator, num_slots: AtomicUsize::new(num_slots), storage: Arc::new(storage) })
        
    }

    /// Writes data to a specific cluster
    ///
    /// # Arguments
    /// * `cluster_id` - Target cluster identifier
    /// * `data` - Data to write (must match cluster size)
    ///
    /// # Returns
    /// * `Result<()>` - Success or IO error
    pub async fn write(&self, cluster_id: usize, data: &[u8]) -> Result<()> {
        assert!(data.len() == self.config.cluster_size);

        
        let slot = self.allocator.pop().await;
        let mut storage = File::from_std(self.storage.try_clone()?);

        let raw_offset = slot as u64 * self.config.cluster_size as u64;
    
        storage.seek(SeekFrom::Start(raw_offset)).await?;
        storage.write_all(data).await?;
        storage.flush().await?;

        custom_sync_range(self.storage.clone(), raw_offset, data.len() as u64).await?;

        let (snapshot_pending, dec_offset) = {
            let offset_table = self.offset_table.read().await;


            let snapshot_pending = offset_table.snapshot_pending;
            let hashmap = &offset_table.inner;

            let snapshot = hashmap.get(&snapshot_pending).unwrap();

            
            let mut entry = snapshot.get(cluster_id).unwrap().lock().await;


            let dec_offset = (entry.db_snapshot == snapshot_pending as u64).then_some(entry.offset as usize);

            *entry = OffsetTableEntry { db_snapshot: snapshot_pending as u64, offset: slot as u64 };

            (snapshot_pending, dec_offset)
        };

        self.db.set_offset(snapshot_pending, cluster_id, slot)?;
        
        if let Some(offset) = dec_offset {
            self.allocator.dec(offset).await;
        }

        let num_slots = self.allocator.len().await;
        let old_num_slots = self.num_slots.swap(num_slots, std::sync::atomic::Ordering::Relaxed);
        if old_num_slots != num_slots {
            self.db.set_num_slots(num_slots)?;
        }

        self.db.flush()?;
        
        Ok(())
    }

    /// Reads entire cluster data from a specific snapshot
    ///
    /// # Arguments
    /// * `snapshot` - Snapshot identifier
    /// * `cluster_id` - Target cluster identifier
    ///
    /// # Returns
    /// * `Result<Vec<u8>>` - Cluster data or IO error
    pub async fn read(&self, snapshot: usize, cluster_id: usize) -> Result<Vec<u8>> {
        self.read_exact(snapshot, cluster_id, 0, self.config.cluster_size).await
    }

    /// Reads a specific range of data from a cluster in a snapshot
    ///
    /// # Arguments
    /// * `snapshot` - Snapshot identifier
    /// * `cluster_id` - Target cluster identifier
    /// * `from` - Start offset within the cluster
    /// * `len` - Number of bytes to read
    ///
    /// # Returns
    /// * `Result<Vec<u8>>` - Requested data or IO error
    pub async fn read_exact(&self, snapshot: usize, cluster_id: usize, from:usize, len:usize) -> Result<Vec<u8>> {
        let offset_table = self.offset_table.read().await;
        let offset = offset_table.inner.get(&snapshot).unwrap().get(cluster_id).unwrap().lock().await.offset as usize;
        let mut storage = File::from_std(self.storage.try_clone()?);
        storage.seek(SeekFrom::Start(offset as u64 * self.config.cluster_size as u64 + from as u64)).await.unwrap();
        let mut data = vec![0; len];
        storage.read_exact(&mut data).await?;
        Ok(data)
    }

    /// Creates a new snapshot of the current state
    ///
    /// # Returns
    /// * `Result<()>` - Success or IO error
    pub async fn add_snapshot(&self) -> Result<()> {
        let pending = {
            let mut offset_table = self.offset_table.write().await;
            let old_pending_snapshot_id = offset_table.snapshot_pending;
            let old_pending_snapshot = offset_table.inner.remove(&old_pending_snapshot_id).unwrap();

            let old_pending_snapshot_values = mutex_vec_values(old_pending_snapshot);
            let old_pending_snapshot = to_mutex_vec(&old_pending_snapshot_values);
            let cloned_pending_snapshot = to_mutex_vec(&old_pending_snapshot_values);

            let slots_to_inc = old_pending_snapshot_values.iter().map(|item| item.offset as usize).collect::<Vec<_>>();

            offset_table.inner.insert(old_pending_snapshot_id, old_pending_snapshot);
            offset_table.inner.insert(old_pending_snapshot_id+1, cloned_pending_snapshot);
            offset_table.snapshot_pending += 1;

            self.allocator.inc_many(&slots_to_inc).await;


            offset_table.snapshot_pending
        };

        self.db.set_snapshot_pending(pending)?;
        Ok(())
    }

    /// Finalizes and removes the oldest snapshot
    ///
    /// # Returns
    /// * `Result<()>` - Success or IO error
    pub async fn join_snapshot(&self) -> Result<()> {
        let (removed_snapshot, removed_snapshot_id, offset_table) = {
            let mut offset_table = self.offset_table.write().await;
            let removed_snapshot_id = offset_table.snapshot_start;
            let removed_snapshot = offset_table.inner.remove(&removed_snapshot_id).unwrap();
            offset_table.snapshot_start += 1; 
            (removed_snapshot, removed_snapshot_id, offset_table.downgrade())
        };

        let start = removed_snapshot_id+1;
        let mut keys_to_remove = vec![];
        let mut offsets_to_dec = vec![];

        let start_snapshot = offset_table.inner.get(&start).unwrap();

        for (cluster_id, entry) in removed_snapshot.into_iter().enumerate() {
            let entry = entry.into_inner();
            let start_entry = start_snapshot.get(cluster_id).unwrap().lock().await.clone();
            if start_entry.db_snapshot == start as u64 {
                offsets_to_dec.push(entry.offset as usize);
                keys_to_remove.push(SledKey::OffsetTable(entry.db_snapshot as u64, cluster_id as u64));
            }
        }

        drop(offset_table);

        self.allocator.dec_many(&offsets_to_dec).await;
        self.db.remove_keys(&keys_to_remove)?;
        self.db.flush()?;

        Ok(())
    }
}


async fn init_db(db: &SledWrapper, fp: &StdFile, config: &SnapshotDbConfig) -> Result<()> {
    db.set_snapshot_start(0)?;
    db.set_snapshot_pending(1)?;
    db.set_num_slots(FREE_SLOTS_MIN_RESERVE + config.num_clusters)?;

    for i in 0..config.num_clusters {
        db.set_offset(0, i, i)?;
    }

    db.flush()?;

    // TODO: replace with default values
    fp.set_len(0)?;
    fp.set_len(config.cluster_size as u64 * config.num_clusters as u64)?;

    Ok(())
}


fn init_offset_table(db: &SledWrapper, config: &SnapshotDbConfig) -> Result<Vec<Vec<Option<OffsetTableEntry>>>> {
    
    let start = db.get_snapshot_start()?;
    let pending = db.get_snapshot_pending()?;

    let mut offset_table = Vec::new();

    for _ in start..=pending {
        offset_table.push(vec![None; config.num_clusters]);
    }


    let mut keys_to_remove = Vec::new();

    let mut computed_pending = pending;

    for (k, offset) in db.offset_table_entries_iter() {
        if let SledKey::OffsetTable(db_snapshot, cluster_id) = k {
            if db_snapshot as usize > computed_pending {
                computed_pending = db_snapshot as usize;
            }

            if (db_snapshot as usize) < start {
                if offset_table[0][cluster_id as usize].map_or(true, |item: OffsetTableEntry| item.db_snapshot < db_snapshot) {
                    if let Some(item) = offset_table[0][cluster_id as usize] {
                        keys_to_remove.push(SledKey::OffsetTable(item.db_snapshot, cluster_id));
                    }

                    offset_table[0][cluster_id as usize] = Some(OffsetTableEntry { db_snapshot, offset: offset as u64 });
                }
            } else {
                offset_table[db_snapshot as usize - start][cluster_id as usize] = Some(OffsetTableEntry { db_snapshot, offset: offset as u64 });
            }
        }
    }

    db.remove_keys(&keys_to_remove)?;

    for i in 1..=pending-start {
        for j in 0..config.num_clusters {
            if offset_table[i][j].is_none() {
                offset_table[i][j] = offset_table[i-1][j];
            }
        }
    }

    if computed_pending != pending {
        db.set_snapshot_pending(computed_pending)?;
    }


    Ok(offset_table)
}

