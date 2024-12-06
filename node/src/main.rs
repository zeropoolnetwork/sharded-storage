use std::{
    future::IntoFuture,
    hash::{Hash, Hasher},
    sync::Arc,
};

use clap::Parser;
use color_eyre::eyre::Result;
use common::{config::StorageConfig, crypto::derive_keys};
use libp2p::{futures::StreamExt, swarm::NetworkBehaviour};
use m31jubjub::hdwallet::{priv_key, pub_key};
use serde::Serialize;
use snapshot_db::db::{SnapshotDb, SnapshotDbConfig};

use crate::state::{AppState, NodeId};

mod api;
mod network;
mod state;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'a', long)]
    api_addr: Option<String>,
    #[arg(short = 'u', long)]
    public_api_url: Option<String>,
    #[arg(short = 'p', long)]
    p2p_port: Option<u16>,
    #[arg(short = 'b', long)]
    boot_node: Option<String>,
    #[arg(long)]
    seed_phrase: Option<String>,
    #[arg(long)]
    node_id: Option<NodeId>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let storage_dir = std::env::var("STORAGE_DIR").unwrap_or_else(|_| "./data/storage".to_string());
    let api_addr = args
        .api_addr
        .or(std::env::var("API_ADDR").ok())
        .unwrap_or_else(|| "0.0.0.0:3000".to_string());
    let public_api_url = args
        .public_api_url
        .or(std::env::var("PUBLIC_API_URL").ok())
        .expect("Public API URL not set");
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
    let node_id = args
        .node_id
        .or_else(|| {
            std::env::var("NODE_ID")
                .ok()
                .map(|id| id.parse::<NodeId>().unwrap())
        })
        .expect("Node ID not set");
    let boot_node = args
        .boot_node
        .or(std::env::var("BOOT_NODE").ok())
        .map(|addr| addr.parse().expect("Invalid boot node address"));

    let (sk, pk) = derive_keys(&seed_phrase).expect("Invalid seed phrase");

    let network_config = network::Config {
        p2p_port,
        boot_node,
        node_id,
        public_api_url,
    };

    let storage_config = StorageConfig::dev();
    let db_config = SnapshotDbConfig {
        cluster_size: storage_config.cluster_size_bytes(),
        num_clusters: storage_config.num_clusters(),
    };
    let storage = SnapshotDb::new(&storage_dir, db_config).await?;

    let state = Arc::new(AppState::new(storage, sk, pk));

    let http_server = api::start_server(state.clone(), &api_addr);
    tokio::pin!(http_server);
    let network = network::start_network(network_config, state);
    tokio::pin!(network);

    tokio::select! {
        res = http_server => { res? },
        res = network => { res? },
    }

    Ok(())
}
