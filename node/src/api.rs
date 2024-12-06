use std::{collections::HashSet, net::SocketAddr, ops::Deref, sync::Arc};

use axum::{
    extract::{Multipart, Path},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use color_eyre::Result;
use serde_json::json;

use crate::state::AppState;

async fn download_cluster(
    state: axum::extract::State<Arc<AppState>>,
    Path(id): Path<usize>,
) -> Result<Response, StatusCode> {
    let data = state
        .storage
        .read(0, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let body = axum::body::Body::from(data);
    let headers = [(header::CONTENT_TYPE, "application/octet-stream")];

    Ok((headers, body).into_response())
}

async fn get_info(state: axum::extract::State<Arc<AppState>>) -> Json<serde_json::Value> {
    // TODO: Get rid of locks in public API
    let peers = state.peers.read().await.clone();
    Json(json!({
        "peers": peers
    }))
}

pub async fn start_server(state: Arc<AppState>, addr: &str) -> Result<()> {
    let app = Router::new()
        .route("/cluster/:id", get(download_cluster))
        .route("/info", get(get_info))
        .with_state(state.clone());

    let addr: SocketAddr = addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("HTTP server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
