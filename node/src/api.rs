use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use serde_json::json;
use tokio::sync::Mutex;

use crate::storage;

pub struct AppState {
    pub peers: Arc<Mutex<HashSet<String>>>,
    pub storage: storage::Storage,
}

async fn upload_sector(
    state: axum::extract::State<Arc<AppState>>,
    mut multipart: Multipart,
) -> color_eyre::Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let _name = field.name().unwrap().to_string();
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let elements = bincode::deserialize(&data).map_err(|_| StatusCode::BAD_REQUEST)?;

        state
            .storage
            .write(
                elements,
                file_name.parse().map_err(|_| StatusCode::BAD_REQUEST)?,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok((StatusCode::CREATED, Json(json!({ "status": "ok" }))));
    }

    Err(StatusCode::BAD_REQUEST)
}

async fn download_sector(
    state: axum::extract::State<Arc<AppState>>,
    Path(sector_id): Path<usize>,
) -> color_eyre::Result<String, StatusCode> {
    let data = state
        .storage
        .read(sector_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let bin_data = bincode::serialize(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let base64_data = BASE64_STANDARD.encode(&bin_data);

    Ok(base64_data)
}

async fn info_handler(state: axum::extract::State<Arc<AppState>>) -> Json<serde_json::Value> {
    let peers = state.peers.lock().await;
    Json(json!({
        "peers": peers.iter().collect::<Vec<_>>()
    }))
}

pub async fn start_server(state: Arc<AppState>, addr: &str) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/sectors", post(upload_sector))
        .route("/sectors/:id", get(download_sector))
        .route("/info", get(info_handler))
        .with_state(state.clone());

    let addr: SocketAddr = addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
