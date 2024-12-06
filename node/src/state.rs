use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use libp2p::{Multiaddr, PeerId};
use m31jubjub::m31::{Fq, Fs};
use serde::{Deserialize, Serialize};
use snapshot_db::db::SnapshotDb;
use tokio::sync::{Mutex, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    pub peer_id: PeerId,
    pub addr: Multiaddr,
    pub api_url: String,
}

pub type NodeId = u32;

pub struct AppState {
    // We don't need to use kademlia since our routing table is pretty small, and we need to access
    // nodes by their ID with as little delay as possible. So storing the full routing table on each
    // node with some ad-hoc replication is acceptable for now.
    /// Routing table that also contains some node metadata.
    pub peers: RwLock<HashMap<NodeId, Peer>>,
    pub validators: RwLock<Vec<Peer>>,
    pub storage: SnapshotDb,
    pub sk: Fs,
    pub pk: Fq,
}

impl AppState {
    pub fn new(storage: SnapshotDb, sk: Fs, pk: Fq) -> Self {
        Self {
            peers: RwLock::default(),
            validators: RwLock::default(),
            storage,
            sk,
            pk,
        }
    }
}
