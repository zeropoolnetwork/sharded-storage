use std::{
    net::SocketAddr,
    future::IntoFuture,
    collections::HashSet,
    path::PathBuf,
    sync::Arc,
    time::Duration
};
use color_eyre::eyre::Result;
use libp2p::{identity, mdns, noise, swarm::{NetworkBehaviour, SwarmEvent}, tcp, yamux, PeerId};
use libp2p::futures::{select, StreamExt};
use axum::{
    extract::Multipart,
    routing::{get, post},
    Router, Json,
};
use axum::http::StatusCode;
use clap::Parser;
use serde_json::json;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

mod api;
mod storage;

#[derive(NetworkBehaviour)]
struct Behaviour {
    mdns: mdns::tokio::Behaviour,
}

struct Network {
    swarm: libp2p::Swarm<Behaviour>,
    known_peers: Vec<PeerId>,
}

struct AppState {
    peers: Arc<Mutex<HashSet<String>>>,
    storage: storage::Storage,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short = 'a', long)]
    api_addr: Option<String>,
    #[arg(short = 'p', long)]
    p2p_port: Option<u16>,
}


#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // TODO: Proper config support
    let storage_dir = std::env::var("STORAGE_DIR").unwrap_or_else(|_| "./storage".to_string());
    let api_addr = cli.api_addr.unwrap_or_else(|| std::env::var("API_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string()));
    let p2p_port = cli.p2p_port.unwrap_or_else(|| std::env::var("P2P_PORT").map(|p| p.parse::<u16>().unwrap()).unwrap_or(4001u16));

    let state = Arc::new(AppState {
        peers: Arc::new(Mutex::new(HashSet::new())),
        storage: storage::Storage::new(storage_dir)?,
    });

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            Ok(Behaviour {
                mdns: mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // for peer in &BOOTNODES {
    //     swarm
    //         .behaviour_mut()
    //         .add_address(&peer.parse()?, "/dnsaddr/bootstrap.libp2p.io".parse()?);
    // }


    swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", p2p_port).parse()?)?;

    let app = Router::new()
        .route("/upload", post(upload_handler))
        .route("/info", get(info_handler))
        .with_state(state.clone());


    tokio::spawn(async move {
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {:?}", address);
                }
                SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, addr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                    }
                }
                _ => {}
            }
        }
    });


    // Run the HTTP server
    let addr: SocketAddr = api_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn upload_handler(
    state: axum::extract::State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let _name = field.name().unwrap().to_string();
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

        state.storage.write_segment(data.to_vec(), file_name.parse().map_err(|_| StatusCode::BAD_REQUEST)?).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok((StatusCode::CREATED, Json(json!({ "status": "ok" }))));
    }

    Err(StatusCode::BAD_REQUEST)
}

async fn info_handler(state: axum::extract::State<Arc<AppState>>) -> Json<serde_json::Value> {
    let peers = state.peers.lock().await;
    Json(json!({
        "peers": peers.iter().collect::<Vec<_>>()
    }))
}