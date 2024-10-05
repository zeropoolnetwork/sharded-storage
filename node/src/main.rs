use std::{
    collections::{HashMap, HashSet},
    future::IntoFuture,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
    time::Duration,
};

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use libp2p::{
    futures::{select, StreamExt},
    gossipsub, identify, identity,
    identity::PublicKey,
    kad, noise, request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, StreamProtocol,
};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, sync::Mutex};
use zeropool_sharded_storage_common::config::StorageConfig;

use crate::api::{start_server, AppState};

mod api;
mod storage;

const KAD_PROTO_NAME: StreamProtocol = StreamProtocol::new("/zpss/kad/1.0.0");

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short = 'a', long)]
    api_addr: Option<String>,
    #[arg(short = 'p', long)]
    p2p_port: Option<u16>,
    #[arg(short, long)]
    boot_nodes: Option<Vec<String>>,
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    // mdns: mdns::tokio::Behaviour,
    // identity: identify::Behaviour,
    // TODO: Can we avoid using kad for our use case (all nodes must know about each other)
    kad: kad::Behaviour<kad::store::MemoryStore>,
    // request_response: request_response::cbor::Behaviour<NodeRequest, NodeResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
enum NodeRequest {
    GetChunk { index: usize },
}
#[derive(Debug, Serialize, Deserialize)]
enum NodeResponse {
    Chunk(Vec<u8>),
}

enum NodeMessage {}

struct Network {
    swarm: libp2p::Swarm<Behaviour>,
    peers: HashMap<PeerId, Peer>,
}

struct Peer {
    addr: Multiaddr,
    public_key: Option<PublicKey>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // TODO: Proper config support
    let storage_dir = std::env::var("STORAGE_DIR").unwrap_or_else(|_| "./storage".to_string());
    let api_addr = cli.api_addr.unwrap_or_else(|| {
        std::env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string())
    });
    let p2p_port = cli.p2p_port.unwrap_or_else(|| {
        std::env::var("P2P_PORT")
            .map(|p| p.parse::<u16>().unwrap())
            .unwrap_or(4001u16)
    });

    let storage_config = StorageConfig::dev();

    let state = Arc::new(AppState {
        peers: Arc::new(Mutex::new(HashSet::new())),
        storage: storage::Storage::new(storage_dir, storage_config)?,
    });

    let key = identity::Keypair::generate_ed25519();

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        // .with_tcp(
        //     tcp::Config::default(),
        //     noise::Config::new,
        //     yamux::Config::default,
        // )?
        .with_quic()
        .with_behaviour(|key| {
            let mut cfg = kad::Config::new(KAD_PROTO_NAME);
            cfg.set_query_timeout(Duration::from_secs(5 * 60));
            let store = kad::store::MemoryStore::new(key.public().to_peer_id());
            let kademlia = kad::Behaviour::with_config(key.public().to_peer_id(), store, cfg);

            Ok(Behaviour {
                // mdns: mdns::tokio::Behaviour::new(
                //     mdns::Config::default(),
                //     key.public().to_peer_id(),
                // )?,
                // identity: identify::Behaviour::new(identify::Config::new(
                //     "/ipfs/id/1.0.0".to_string(),
                //     key.public(),
                // )),
                kad: kademlia,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    if let Some(boot_nodes) = cli.boot_nodes {
        for addr in boot_nodes {
            let remote: Multiaddr = addr.parse()?;
            swarm.dial(remote)?;
        }
    }

    // swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", p2p_port).parse()?)?;
    swarm.listen_on(format!("/ip4/0.0.0.0/udp/{}/quic-v1", p2p_port).parse()?)?;

    let mut network = Network {
        swarm,
        peers: HashMap::new(),
    };

    tokio::spawn(async move {
        loop {
            match network.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    tracing::info!("Listening on {:?}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    tracing::debug!("Connected to {peer_id}");
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    tracing::debug!("Disconnected from {peer_id}");
                }
                SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        tracing::info!("mDNS discovered a new peer: {peer_id}");
                        // swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                // SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                //     for (peer_id, _multiaddr) in list {
                //         tracing::info!("mDNS discover peer has expired: {peer_id}");
                //         swarm
                //             .behaviour_mut()
                //             .gossipsub
                //             .remove_explicit_peer(&peer_id);
                //     }
                // }
                // SwarmEvent::Behaviour(BehaviourEvent::Identity(identify::Event::Sent {
                //     peer_id,
                //     ..
                // })) => {
                //     tracing::debug!("Sent identify info to {peer_id:?}");
                // }
                // SwarmEvent::Behaviour(BehaviourEvent::Identity(identify::Event::Received {
                //     info,
                //     peer_id,
                //     ..
                // })) => {
                //     tracing::debug!("Received identity info {info:?}");
                //     network.peers.insert(
                //         peer_id,
                //         Peer {
                //             addr: info.observed_addr,
                //             public_key: Some(info.public_key),
                //         },
                //     );
                // }
                _ => {}
            }
        }
    });

    start_server(state, &api_addr).await?;

    Ok(())
}
