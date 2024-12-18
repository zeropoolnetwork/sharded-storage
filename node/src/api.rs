use std::{collections::HashSet, net::SocketAddr, ops::Deref, sync::Arc};

use axum::{
    extract::{Multipart, Path},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use color_eyre::Result;
use p3_matrix::dense::RowMajorMatrix;
use primitives::Val;
use serde_json::json;
use shards::compute_commitment;

use crate::state::{AppState, Command, NodeState};

async fn download_cluster(
    state: axum::extract::State<Arc<AppState>>,
    Path(cluster_id): Path<usize>,
) -> Result<Response, StatusCode> {
    match &state.node_state {
        NodeState::Validator => Err(StatusCode::FORBIDDEN),
        NodeState::Storage { storage } => {
            let data = storage
                .read(0, cluster_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let body = axum::body::Body::from(data);
            let headers = [(header::CONTENT_TYPE, "application/octet-stream")];

            Ok((headers, body).into_response())
        }
    }
}

async fn upload_cluster(
    state: axum::extract::State<Arc<AppState>>,
    Path(cluster_id): Path<u32>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let elements: Vec<Val> =
            bincode::deserialize(&data).map_err(|_| StatusCode::BAD_REQUEST)?;
        let matrix = RowMajorMatrix::new(elements, state.storage_config.m);
        let (commit, shards) = compute_commitment(matrix, state.storage_config.log_blowup_factor());

        state
            .command_sender
            .send(Command::UploadCluster {
                id: cluster_id,
                shards,
            })
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok((StatusCode::CREATED, Json(json!({ "status": "ok" }))));
    }

    Err(StatusCode::BAD_REQUEST)
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
        .route(
            "/cluster/:cluster_id",
            get(download_cluster).post(upload_cluster),
        )
        .route("/info", get(get_info))
        .route("/", get(get_info))
        .with_state(state.clone());

    let addr: SocketAddr = addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("HTTP server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
