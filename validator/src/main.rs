use std::{collections::HashSet, fmt::Display, net::SocketAddr, sync::Arc};

use axum::{
    extract::Multipart,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tokio::sync::Mutex;

use crate::storage;

pub struct AppState {}

async fn upload_handler(
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

        state
            .storage
            .write_sector(
                data.to_vec(),
                file_name.parse().map_err(|_| StatusCode::BAD_REQUEST)?,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        return Ok((StatusCode::CREATED, Json(json!({ "status": "ok" }))));
    }

    Err(StatusCode::BAD_REQUEST)
}

async fn info_handler(state: axum::extract::State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "OK",
    }))
}

pub async fn start_server(state: Arc<AppState>, addr: &str) -> color_eyre::Result<()> {
    let app = Router::new()
        .route("/upload", post(upload_handler))
        .route("/info", get(info_handler))
        .with_state(state.clone());

    let addr: SocketAddr = addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Display>> {
    start_server().await?;
}
