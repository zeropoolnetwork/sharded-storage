use std::{collections::HashSet, net::SocketAddr, ops::Deref, sync::Arc};

use axum::{
    extract::{Multipart, Path},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use color_eyre::Result;
use common::{contract::ClusterId, crypto::verify, encode::encode_aligned, node::UploadMessage};
use m31jubjub::{eddsa::SigParams, m31::M31JubJubSigParams};
use p3_matrix::dense::RowMajorMatrix;
use primitives::Val;
use serde_json::json;
use shards::compute_commitment;

use crate::state::{AppState, Command, NodeState};

#[tracing::instrument(skip(state), level = "info")]
async fn download_cluster(
    state: axum::extract::State<Arc<AppState>>,
    Path(cluster_id): Path<String>,
) -> Result<Response, StatusCode> {
    let cluster_id: ClusterId = cluster_id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let cached_cluster_index = state.cluster_id_cache.read().await.get(&cluster_id).cloned();
    let cluster_index = if let Some(index) = cached_cluster_index {
        index
    } else {
        let metadata = state
            .contract_client
            .get_cluster(&cluster_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let index = metadata.index as usize;
        state
            .cluster_id_cache
            .write()
            .await
            .insert(cluster_id.clone(), index);
        index
    };

    match &state.node_state {
        NodeState::Validator => Err(StatusCode::FORBIDDEN),
        NodeState::Storage { storage } => {
            let data = storage
                .read(1, cluster_index)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let body = axum::body::Body::from(data);
            let headers = [(header::CONTENT_TYPE, "application/octet-stream")];

            Ok((headers, body).into_response())
        }
    }
}

#[tracing::instrument(skip(state, multipart), level = "info")]
async fn upload_cluster(
    state: axum::extract::State<Arc<AppState>>,
    Path(cluster_id): Path<String>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let cluster_id = cluster_id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let cluster_metadata = state
            .contract_client
            .get_cluster(&cluster_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        let msg: UploadMessage =
            bincode::deserialize(&data).map_err(|_| StatusCode::BAD_REQUEST)?;

        let elements = encode_aligned(&msg.data, state.storage_config.cluster_size())
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        if !verify(&elements, msg.signature, cluster_metadata.owner_pk) {
            tracing::debug!("Invalid signature");
            return Err(StatusCode::BAD_REQUEST);
        }

        let matrix = RowMajorMatrix::new(elements, state.storage_config.m);
        let (commit, shards) = compute_commitment(matrix, state.storage_config.log_blowup_factor());

        if cluster_metadata.commit != commit.pcs_commitment_hash {
            tracing::debug!("Invalid commit");
            return Err(StatusCode::BAD_REQUEST);
        }

        state
            .command_sender
            .send(Command::UploadCluster {
                index: cluster_metadata.index,
                id: cluster_id.clone(),
                shards,
            })
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok((StatusCode::CREATED, Json(json!({ "status": "ok" }))));
    }

    Err(StatusCode::BAD_REQUEST)
}

#[tracing::instrument(skip(state), level = "info")]
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
            "/clusters/:cluster_id",
            get(download_cluster).post(upload_cluster),
        )
        .route("/info", get(get_info))
        .route("/", get(get_info))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state.clone());

    let addr: SocketAddr = addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("HTTP server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
