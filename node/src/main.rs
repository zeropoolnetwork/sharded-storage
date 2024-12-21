use std::{
    future::IntoFuture,
    hash::{Hash, Hasher},
    sync::Arc,
};

use clap::Parser;
use color_eyre::eyre::Result;
use common::{config::StorageConfig, contract::MockContractClient, crypto::derive_keys};
use libp2p::{futures::StreamExt, swarm::NetworkBehaviour};
use m31jubjub::hdwallet::{priv_key, pub_key};
use primitives::Val;
use serde::Serialize;
use snapshot_db::db::{SnapshotDb, SnapshotDbConfig};

use crate::state::{AppState, NodeId, NodeKind, NodeState};

mod api;
mod network;
mod state;

// TODO: Might want to extract the validator into a separate crate in the future.
// TODO: I'm not sure if libp2p is even needed here: we're only using it for transport, encryption,
//       request/response.

const COMMAND_CHANNEL_CAPACITY: usize = 100;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'a', long)]
    api_addr: Option<String>,
    #[arg(short = 'u', long)]
    public_api_url: Option<String>,
    #[arg(long)]
    external_ip: Option<String>,
    #[arg(short = 'p', long)]
    p2p_port: Option<u16>,
    #[arg(short = 'b', long)]
    boot_node: Option<String>,
    #[arg(long)]
    seed_phrase: Option<String>,
    #[arg(long)]
    node_id: Option<NodeId>,
    #[arg(short = 'c', long)]
    contract_mock_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let api_addr = args
        .api_addr
        .or(std::env::var("API_ADDR").ok())
        .unwrap_or_else(|| "0.0.0.0:3000".to_string());
    let public_api_url = args
        .public_api_url
        .or(std::env::var("PUBLIC_API_URL").ok())
        .expect("Public API URL not set");
    let external_ip = args
        .external_ip
        .or(std::env::var("EXTERNAL_IP").ok())
        .expect("External IP not set");
    let p2p_port = args
        .p2p_port
        .or_else(|| {
            std::env::var("P2P_PORT")
                .ok()
                .map(|p| p.parse::<u16>().unwrap())
        })
        .expect("Port not set");
    let seed_phrase = args
        .seed_phrase
        .or(std::env::var("SEED_PHRASE").ok())
        .expect("Seed phrase not set");
    let node_id = args.node_id.or_else(|| {
        std::env::var("NODE_ID")
            .ok()
            .map(|id| id.parse::<NodeId>().unwrap())
    });
    let boot_node = args
        .boot_node
        .or(std::env::var("BOOT_NODE").ok())
        .map(|addr| addr.parse().expect("Invalid boot node address"));
    let contract_mock_url = args
        .contract_mock_url
        .or(std::env::var("CONTRACT_MOCK_URL").ok())
        .expect("Contract mock URL not set");

    let node_kind = match node_id {
        Some(id) => NodeKind::Storage { id },
        None => NodeKind::Validator,
    };

    let (sk, pk) = derive_keys(&seed_phrase).expect("Invalid seed phrase");

    let network_config = network::Config {
        p2p_port,
        boot_node,
        node_kind: node_kind.clone(),
        public_api_url,
        external_ip,
    };

    let storage_config = StorageConfig::dev();

    let node_state = match node_kind {
        NodeKind::Validator => NodeState::Validator,
        NodeKind::Storage { id } => {
            let db_config = SnapshotDbConfig {
                cluster_size: storage_config.shard_size() * size_of::<Val>(), // FIXME: size of shard
                num_clusters: storage_config.num_clusters(),
            };
            let storage_dir =
                std::env::var("STORAGE_DIR").unwrap_or_else(|_| "./data/storage".to_string());
            let storage = SnapshotDb::new(&storage_dir, db_config).await?;
            NodeState::Storage { storage }
        }
    };

    let contract_client = MockContractClient::new(&contract_mock_url);

    let (command_sender, command_receiver) = tokio::sync::mpsc::channel(COMMAND_CHANNEL_CAPACITY);
    let state = Arc::new(AppState::new(
        sk,
        pk,
        storage_config,
        command_sender,
        node_state,
        contract_client,
    ));

    let http_server = api::start_server(state.clone(), &api_addr);
    tokio::pin!(http_server);
    let network = network::start_network(network_config, state, command_receiver);
    tokio::pin!(network);

    tokio::select! {
        res = http_server => { res? },
        res = network => { res? },
    }

    Ok(())
}
