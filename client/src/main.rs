use std::fs;
use clap::{Parser, Subcommand};
use reqwest::multipart;
use serde_json::Value;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt}
};
use zeropool_sharded_storage_common::blowup;
use zeropool_sharded_storage_common::config::StorageConfig;
use zeropool_sharded_storage_common::encode::encode;

// TODO: Proper logging
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, default_value = "http://localhost:3000")]
    node_url: String,
}

#[derive(Subcommand)]
enum Commands {
    Upload {
        #[arg(short, long)]
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let storage_config = serde_json::from_str(&fs::read_to_string("storage_config.json")?);

    match cli.command {
        Commands::Upload { file } => {
            upload_file(file, &cli.node_url, &storage_config).await?;
        }
    }

    Ok(())
}

async fn upload_file(file_path: PathBuf, node: &str, storage_config: &StorageConfig) -> Result<(), Box<dyn std::error::Error>> {
    // let mut file = File::open(file_path).await?;
    // let file_size = file.metadata().await?.len();
    let file_data = fs::read(&file_path)?;

    let sector_size = storage_config.n * storage_config.m;
    let blowup_factor = storage_config.q / storage_config.m;

    let client = reqwest::Client::new();

    let nodes_response: Value = client
        .get(&format!("{}/info", node))
        .send()
        .await?
        .json()
        .await?;

    // FIXME: Proper types
    let nodes = nodes_response["peers"].as_array().unwrap();
    if nodes.len() < blowup_factor {
        return Err("Not enough nodes to upload the file".into());
    }

    let encoded_file = encode(&file_data);
    let blown_up_sectors = encoded_file.chunks(sector_size).map(|sector| blowup(sector, blowup_factor));

    // FIXME: Allocate sectors beforehand
    for (sector_index, sector) in blown_up_sectors.enumerate() {
        let sector_shards = sector.chunks(storage_config.n);

        for shard in sector_shards {
            let shard_data = bincode::serialize(shard)?;
            let node_url = nodes[sector_index as usize]["address"].as_str().unwrap();
            upload_shard(&client, sector_index, node_url, shard_data).await?;
        }
    }


    println!("File uploaded successfully!");
    Ok(())
}

async fn upload_shard(
    client: &reqwest::Client,
    index: usize,
    node_url: &str,
    data: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let part = multipart::Part::bytes(data)
        .file_name(index.to_string())
        .mime_str("application/octet-stream")?;

    let form = multipart::Form::new().part("file", part);

    let response = client
        .post(&format!("{}/upload", node_url))
        .multipart(form)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Uploaded chunk {} to {}", index, node_url);
        Ok(())
    } else {
        Err(format!("Failed to upload chunk {} to {}", index, node_url).into())
    }
}