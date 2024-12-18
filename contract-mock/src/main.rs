use std::{collections::HashMap, fmt::Display, net::SocketAddr, sync::Arc};

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use color_eyre::eyre::Result;
use common::crypto::PublicKey;
use primitives::{Hash, Val};
use rand::{random, Rng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

type SlotId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Slot {
    owner_pk: PublicKey,
    segments: Vec<SegmentId>,
}
type SegmentId = [Val; 8];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Segment {
    slot: SlotId,
}

pub struct AppState {
    // Assume that we have a single volume for now.
    slots: RwLock<Vec<Slot>>,
    segments: RwLock<HashMap<SegmentId, Segment>>,
}

#[derive(Deserialize)]
struct UploadSegmentReq {
    segment: SegmentId,
    slot: SlotId,
    owner_pk: PublicKey,
    commit: Hash,
}

// Prepares segment for upload
async fn upload_segment(
    state: axum::extract::State<Arc<AppState>>,
    form: Json<UploadSegmentReq>,
) -> Result<(), StatusCode> {
    let mut slots = state.slots.write().await;

    let mut slot = slots
        .iter_mut()
        .find(|slot| slot.owner_pk == form.owner_pk)
        .ok_or(StatusCode::BAD_REQUEST)?;

    slot.segments.push(form.segment);

    Ok(())
}

async fn slot_segments(
    state: axum::extract::State<Arc<AppState>>,
    axum::extract::Path(slot_id): axum::extract::Path<SlotId>,
) -> Result<Json<Vec<SegmentId>>, StatusCode> {
    let slot = state
        .slots
        .read()
        .await
        .get(slot_id as usize)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    let segments = slot.segments.clone();

    Ok(Json(segments.clone()))
}

async fn info_handler(state: axum::extract::State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "OK",
    }))
}

pub async fn start_server(state: Arc<AppState>, addr: &str) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/info", get(info_handler))
        .route("/slots/:id/segments", get(slot_segments))
        .route("/slots", post(reserve_slot))
        .route("/reserve-segment", post(upload_segment))
        .with_state(state.clone());

    let addr: SocketAddr = addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn reserve_slot(
    state: axum::extract::State<Arc<AppState>>,
    form: Json<Slot>,
) -> Result<Json<u64>, StatusCode> {
    let mut slots = state.slots.write().await;
    let next_slot = slots.len();
    slots.push(form.0.clone());

    Ok((next_slot as u64).into())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        slots: Default::default(),
        segments: Default::default(),
    });
    start_server(state, "0.0.0.0:80").await?;

    Ok(())
}
