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

use crate::{state::AppState, storage};

// async fn upload_sector(
//     state: axum::extract::State<Arc<AppState>>,
//     mut multipart: Multipart,
// ) -> color_eyre::Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
//     while let Some(field) = multipart
//         .next_field()
//         .await
//         .map_err(|_| StatusCode::BAD_REQUEST)?
//     {
//         let _name = field.name().unwrap().to_string();
//         let file_name = field.file_name().unwrap().to_string();
//         let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
//         let elements: Vec<_> =
//             bincode::deserialize(data.as_ref()).map_err(|_| StatusCode::BAD_REQUEST)?;
//
//         state
//             .storage
//             .write(
//                 &elements,
//                 file_name.parse().map_err(|_| StatusCode::BAD_REQUEST)?,
//             )
//             .await
//             .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//
//         return Ok((StatusCode::CREATED, Json(json!({ "status": "ok" }))));
//     }
//
//     Err(StatusCode::BAD_REQUEST)
// }

async fn download_cluster(
    state: axum::extract::State<Arc<AppState>>,
    Path(id): Path<usize>,
) -> color_eyre::Result<Response, StatusCode> {
    let data = state
        .storage
        .read_bin(id)
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
    axum::serve(listener, app).await?;

    Ok(())
}
