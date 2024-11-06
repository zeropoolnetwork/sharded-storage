use std::sync::Arc;

use color_eyre::eyre::Result;

mod api;
mod config;
mod network;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let state = Arc::new(api::AppState {});
    api::start_server(state, "0.0.0.0:80").await?;

    Ok(())
}
