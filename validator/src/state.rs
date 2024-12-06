use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use common::config::StorageConfig;
use libp2p::{Multiaddr, PeerId};
use m31jubjub::m31::{Fq, Fs};
use primitives::Val;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Peer {
    pub peer_id: PeerId,
    pub addr: Multiaddr,
    pub api_url: String,
}

pub type NodeId = u32;

pub enum Command {
    UploadCluster { id: u32, shards: Vec<Vec<Val>> },
}

pub struct AppState {
    pub peers: RwLock<HashMap<NodeId, Peer>>,
    pub validators: RwLock<Vec<Peer>>,
    pub sk: Fs,
    pub pk: Fq,
    pub storage_config: StorageConfig,
    pub command_sender: mpsc::Sender<Command>,
}

impl AppState {
    pub fn new(
        sk: Fs,
        pk: Fq,
        storage_config: StorageConfig,
        command_sender: mpsc::Sender<Command>,
    ) -> Self {
        Self {
            peers: RwLock::default(),
            validators: RwLock::default(),
            sk,
            pk,
            storage_config,
            command_sender,
        }
    }
}
