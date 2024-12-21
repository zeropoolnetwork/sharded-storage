use std::{collections::HashMap, fmt::Display, net::SocketAddr, ops::Deref, sync::Arc};

use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use color_eyre::eyre::Result;
use common::{
    contract::{Cluster, ClusterId},
    crypto::PublicKey,
};
use primitives::Hash;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

const STATE_PATH: &str = "data/contract_mock_state.bin";

#[derive(Clone, Serialize, Deserialize)]
pub struct AppState {
    clusters: Vec<Cluster>,
    cluster_indices: HashMap<ClusterId, usize>,
}

#[derive(Deserialize)]
struct UploadClusterReq {
    owner_pk: PublicKey,
    commit: Hash,
}

#[derive(Serialize, Deserialize)]
struct UploadClusterRes {
    cluster_id: String,
}

async fn reserve_cluster(
    state: axum::extract::State<Arc<RwLock<AppState>>>,
    form: Json<UploadClusterReq>,
) -> Result<Json<UploadClusterRes>, StatusCode> {
    let cluster_id = ClusterId::random();

    let mut state = state.write().await;
    let cur_cluster_index = state.clusters.len();

    let cluster = Cluster {
        index: cur_cluster_index as u64,
        owner_pk: form.owner_pk,
        commit: form.commit,
    };

    state.clusters.push(cluster);
    state
        .cluster_indices
        .insert(cluster_id.clone(), cur_cluster_index);

    tracing::info!("Reserved cluster {}", cluster_id);

    // dump the state to disk, ok for a mock
    let mut file =
        std::fs::File::create(STATE_PATH).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    bincode::serialize_into(&mut file, state.deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(UploadClusterRes {
        cluster_id: cluster_id.to_string(),
    }))
}

#[axum::debug_handler]
async fn get_cluster(
    state: axum::extract::State<Arc<RwLock<AppState>>>,
    Path(cluster_id): Path<String>,
) -> Result<Json<Cluster>, StatusCode> {
    let state = state.read().await;
    let cluster_id = cluster_id.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let clusters_index = *state
        .cluster_indices
        .get(&cluster_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let cluster: &Cluster = state
        .clusters
        .get(clusters_index)
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(cluster.clone()))
}

async fn info_handler(
    state: axum::extract::State<Arc<RwLock<AppState>>>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "OK",
    }))
}

pub async fn start_server(state: Arc<RwLock<AppState>>, addr: &str) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/info", get(info_handler))
        .route("/clusters", post(reserve_cluster))
        .route("/clusters/:cluster_id", get(get_cluster))
        .layer(tower_http::trace::TraceLayer::new_for_http())
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

    let state_res: Result<AppState> = std::fs::read(STATE_PATH)
        .map_err(|err| color_eyre::eyre::eyre!("Failed to read state from disk: {}", err))
        .and_then(|data| {
            bincode::deserialize(&data)
                .map_err(|err| color_eyre::eyre::eyre!("Failed to deserialize state: {}", err))
        });
    let state = match state_res {
        Ok(state) => {
            tracing::info!("Loaded state from disk. Clusters in state: {}", state.clusters.len());
            state
        },
        Err(err) => {
            tracing::warn!("{}. New state initialized.", err);
            AppState {
                clusters: Vec::new(),
                cluster_indices: HashMap::new(),
            }
        }
    };

    let state = Arc::new(RwLock::new(state));

    tracing::info!("Listening on 0.0.0.0:80");
    start_server(state, "0.0.0.0:80").await?;

    Ok(())
}
