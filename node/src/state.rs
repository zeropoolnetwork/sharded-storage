use std::collections::{HashMap, HashSet};

use common::{config::StorageConfig, contract::MockContractClient};
use libp2p::{Multiaddr, PeerId};
use m31jubjub::m31::{Fq, Fs};
use primitives::Val;
use serde::{Deserialize, Serialize};
use snapshot_db::db::SnapshotDb;
use tokio::sync::{mpsc, RwLock};
use common::contract::ClusterId;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Peer {
    pub peer_id: PeerId,
    pub addr: Multiaddr,
    pub api_url: String,
}

pub type NodeId = u32;

#[derive(Clone, Debug)]
pub enum Command {
    UploadCluster { index: u64, id: ClusterId, shards: Vec<Vec<Val>> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    Validator,
    Storage { id: NodeId },
}

pub enum NodeState {
    Validator,
    Storage { storage: SnapshotDb },
}

pub struct AppState {
    // We don't need to use kademlia since our routing table is pretty small, and we need to access
    // nodes by their ID with as little delay as possible. So storing the full routing table on each
    // node with some ad-hoc replication is acceptable for now.
    pub peers: RwLock<HashMap<NodeId, Peer>>,
    pub validators: RwLock<HashSet<Peer>>,
    pub node_state: NodeState,
    pub sk: Fs,
    pub pk: Fq,
    pub storage_config: StorageConfig,
    pub command_sender: mpsc::Sender<Command>,
    pub contract_client: MockContractClient,
    pub cluster_id_cache: RwLock<HashMap<ClusterId, usize>>,
}

impl AppState {
    pub fn new(
        sk: Fs,
        pk: Fq,
        storage_config: StorageConfig,
        command_sender: mpsc::Sender<Command>,
        node_state: NodeState,
        contract_client: MockContractClient,
    ) -> Self {
        Self {
            peers: Default::default(),
            validators: Default::default(),
            node_state,
            sk,
            pk,
            storage_config,
            command_sender,
            contract_client,
            cluster_id_cache: Default::default(),
        }
    }
}
