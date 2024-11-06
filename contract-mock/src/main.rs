use std::{collections::HashMap, fmt::Display, net::SocketAddr, sync::Arc};

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use color_eyre::eyre::Result;
use rand::{random, Rng};
use serde_json::json;
use tokio::sync::RwLock;

// TODO: UUID encoded as m31 vector. Used everywhere on the outside.
type LogicalSegmentId = Vec<Val>;

// Internal segment address
struct Slot {
    volume: Mersenne31,
    index: Mersenne31,
}

// TODO: owner = hash(pk)
// POSEIDON2_HASH.hash_iter(...)

// TODO: Cluster id is an offset inside a segment.
type ClusterId = u64;

#[derive(Debug, Clone)]
struct Owner {
    owner_pk: Vec<u8>,
}

#[derive(Debug)]
pub struct AppState {
    segments: RwLock<HashMap<ClusterId, Owner>>,
}

async fn reserve_segment(
    state: axum::extract::State<Arc<AppState>>,
    form: Json<Owner>,
) -> Result<u64, StatusCode> {
    let new_segment_id = random();

    state
        .segments
        .write()
        .await
        .insert(new_segment_id, form.0.clone());

    Ok(new_segment_id)
}

async fn info_handler(state: axum::extract::State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "OK",
    }))
}

pub async fn start_server(state: Arc<AppState>, addr: &str) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/reserve-cluster", post(reserve_segment))
        .with_state(state.clone());

    let addr: SocketAddr = addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {});
    start_server(state, "0.0.0.0:80").await?;

    Ok(())
}
