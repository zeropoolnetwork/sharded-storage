use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};

use crate::storage::Storage;

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
    pub storage: Storage,
}

impl AppState {
    pub fn new(storage: Storage) -> Self {
        Self {
            peers: RwLock::new(HashMap::new()),
            storage,
        }
    }
}
