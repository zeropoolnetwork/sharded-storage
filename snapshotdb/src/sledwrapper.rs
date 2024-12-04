use bincode::{serialize, deserialize};
use serde::{Serialize, Deserialize};
use sled::Db;
use std::io::Result;


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OffsetTableEntry {
    pub db_snapshot: u64,
    pub offset: u64,
}

impl OffsetTableEntry {
    pub fn new(db_snapshot: usize, offset: usize) -> Self {
        Self { db_snapshot: db_snapshot as u64, offset: offset as u64 }
    }
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SledKey {
    SnapshotStart,
    SnapshotPending,
    NumSlots,
    OffsetTable(u64,u64)
}

impl SledKey {
    fn prefix_bytes(&self) -> Vec<u8> {
        match self {
            SledKey::SnapshotStart => vec![0],
            SledKey::SnapshotPending => vec![1],
            SledKey::NumSlots => vec![2],
            SledKey::OffsetTable(_, _) => vec![3],
        }
    }

    fn bytes(&self) -> Vec<u8> {
        serialize(self).unwrap()
    }
}



#[derive(Debug, Clone)]
pub struct SledWrapper(Db);


impl SledWrapper {
    pub fn new(db: Db) -> Self {
        Self(db)
    }

    pub fn into_inner(self) -> Db {
        self.0
    }

    pub fn as_inner(&self) -> &Db {
        &self.0
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.0.get(SledKey::SnapshotStart.bytes())?.is_none())
    }

    pub fn get_snapshot_start(&self) -> Result<usize> {
        let buff = self.0.get(SledKey::SnapshotStart.bytes())?.unwrap();
        Ok(u64::from_le_bytes(buff.as_ref().try_into().unwrap()) as usize)
    }

    pub fn set_snapshot_start(&self, snapshot_start: usize) -> Result<()> {
        self.0.insert(SledKey::SnapshotStart.bytes(), &u64::to_le_bytes(snapshot_start as u64))?;
        Ok(())
    }

    pub fn get_snapshot_pending(&self) -> Result<usize> {
        let buff = self.0.get(SledKey::SnapshotPending.bytes())?.unwrap();
        Ok(u64::from_le_bytes(buff.as_ref().try_into().unwrap()) as usize)
    }

    pub fn set_snapshot_pending(&self, snapshot_pending: usize) -> Result<()> {
        self.0.insert(SledKey::SnapshotPending.bytes(), &u64::to_le_bytes(snapshot_pending as u64))?;
        Ok(())
    }


    pub fn get_num_slots(&self) -> Result<usize> {
        let buff = self.0.get(SledKey::NumSlots.bytes())?.unwrap();
        Ok(u64::from_le_bytes(buff.as_ref().try_into().unwrap()) as usize)
    }

    pub fn set_num_slots(&self, num_slots: usize) -> Result<()> {
        self.0.insert(SledKey::NumSlots.bytes(), &u64::to_le_bytes(num_slots as u64))?;
        Ok(())
    }

    pub fn get_offset(&self, db_snapshot: usize,cluster_id: usize) -> Result<Option<usize>> {
        let buff = self.0.get(SledKey::OffsetTable(db_snapshot as u64,cluster_id as u64).bytes())?;
        Ok(buff.map(|b| u64::from_le_bytes(b.as_ref().try_into().unwrap()) as usize))
    }

    pub fn set_offset(&self, db_snapshot: usize,cluster_id: usize, offset: usize) -> Result<()> {
        self.0.insert(SledKey::OffsetTable(db_snapshot as u64, cluster_id as u64).bytes(), &u64::to_le_bytes(offset as u64))?;
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.0.flush()?;
        Ok(())
    }

    // Search all offset table entries in db and return a iterator over all entries
    pub fn offset_table_entries_iter(&self) -> impl Iterator<Item = (SledKey, usize)> {
        let prefix = SledKey::OffsetTable(0, 0).prefix_bytes();

        self.0.scan_prefix(prefix).map(|e|{
            let (k,v) = e.unwrap();
            (deserialize(&k).unwrap(), u64::from_le_bytes(v.as_ref().try_into().unwrap()) as usize)
        })
        
    }

    pub fn remove_keys(&self, keys: &[SledKey]) -> Result<()> {
        for key in keys {
            self.0.remove(key.bytes())?;
        }
        Ok(())
    }

    pub fn remove_key(&self, key: &SledKey) -> Result<()> {
        self.0.remove(key.bytes())?;
        Ok(())
    }

}
