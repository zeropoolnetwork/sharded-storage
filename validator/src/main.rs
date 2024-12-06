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

// TODO: Node and validator share most of the code. We can extract the common parts into a separate crate.

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
    contract_mock_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    let storage_dir = std::env::var("STORAGE_DIR").unwrap_or_else(|_| "./storage".to_string());
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
    let boot_node = args
        .boot_node
        .or(std::env::var("BOOT_NODE").ok())
        .map(|addr| addr.parse().expect("Invalid boot node address"));
    let contract_mock_url = args
        .contract_mock_url
        .or(std::env::var("CONTRACT_MOCK_URL").ok());

    let (sk, pk) = derive_keys(&seed_phrase).expect("Invalid seed phrase");

    let network_config = network::Config {
        p2p_port,
        boot_node,
        public_api_url,
    };

    let storage_config = StorageConfig::dev();

    let (command_sender, command_receiver) = tokio::sync::mpsc::channel(100);

    let state = Arc::new(AppState::new(sk, pk, storage_config, command_sender));

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
