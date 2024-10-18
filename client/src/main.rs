use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};
use m31jubjub::{
    eddsa::SigParams,
    hdwallet::{priv_key, pub_key},
    m31::{FqBase, Fs, M31JubJubSigParams},
};
use rand::{thread_rng, Rng};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::AsyncReadExt;
use zeropool_sharded_storage_common::{
    // blowup,
    config::StorageConfig,
    encode::{decode, encode},
    Field,
};

const KEY_PATH: &str = "m/42/0'/1337'"; // FIXME

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
        #[arg(short, long)]
        sector: usize,
        #[arg(short, long)]
        mnemonic: String,
    },
    Download {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long)]
        size: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let storage_config = serde_json::from_str(&fs::read_to_string("storage_config.json")?)?;

    match cli.command {
        Commands::Upload {
            file,
            sector,
            mnemonic,
        } => {
            upload_file(file, &cli.node_url, &storage_config, sector, &mnemonic).await?;
        }
        Commands::Download { id, output, size } => {
            download_file(id, size, output, &cli.node_url, &storage_config).await?;
        }
    }

    Ok(())
}

async fn upload_file(
    file_path: PathBuf,
    node: &str,
    storage_config: &StorageConfig,
    sector: usize,
    mnemonic: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = StorageConfig::dev(); // FIXME
    let file_data = fs::read(&file_path)?;

    if file_data.len() > storage_config.sector_capacity_bytes() {
        return Err("File is too large".into());
    }

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

    let encoded_file = encode(&file_data)
        .into_iter()
        .chain((0..).map(|_| Field::new(0)))
        .take(config.sector_capacity())
        .collect::<Vec<_>>();

    // TODO: Limit to one sector for now

    // FIXME: commitment

    let sector_shards = encoded_file.chunks(storage_config.num_chunks());
    let sig_params = M31JubJubSigParams::default();
    let private_key = priv_key::<M31JubJubSigParams>(mnemonic, KEY_PATH).unwrap();
    let public_key = pub_key::<M31JubJubSigParams>(mnemonic, KEY_PATH).unwrap();
    let signature = sig_params.sign(&encoded_file, private_key);

    // TODO: Proper sector allocation/reservation
    for (i, shard) in sector_shards.enumerate() {
        let shard_data = bincode::serialize(shard)?;
        let node_url = nodes[sector + i]["address"].as_str().unwrap();
        upload_sector(&client, sector + i, node_url, shard_data).await?;
    }

    println!("File uploaded successfully!");
    Ok(())
}

async fn upload_sector(
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

async fn download_sector(
    client: &reqwest::Client,
    index: usize,
    node_url: &str,
) -> Result<Vec<Field>, Box<dyn std::error::Error>> {
    let response = client
        .get(&format!("{}/download/{}", node_url, index))
        .send()
        .await?;

    if response.status().is_success() {
        let data = response.bytes().await?;
        let decoded_data: Vec<Field> = bincode::deserialize(&data)?;
        Ok(decoded_data)
    } else {
        Err(format!("Failed to download chunk {} from {}", index, node_url).into())
    }
}

async fn download_file(
    file_id: String,
    size: usize,
    output: PathBuf,
    node: &str,
    storage_config: &StorageConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let nodes_response: Value = client
        .get(&format!("{}/info", node))
        .send()
        .await?
        .json()
        .await?;

    let nodes = nodes_response["peers"].as_array().unwrap();
    let blowup_factor = storage_config.q / storage_config.m;
    // let sector_size = storage_config.n * storage_config.m;

    if nodes.len() < blowup_factor {
        return Err("Not enough nodes to download the file".into());
    }

    let mut downloaded_data = Vec::new();
    let mut sector_index = 0;

    loop {
        let mut sector_shards = Vec::new();

        for i in 0..blowup_factor {
            let node_url = nodes[i]["address"].as_str().unwrap();
            match download_sector(&client, sector_index, node_url).await {
                Ok(shard) => {
                    sector_shards.push(shard);
                }
                Err(_) => continue,
            }
        }

        if sector_shards.is_empty() {
            break;
        }

        let values = sector_shards
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<Field>>();
        let reconstructed_sector =
            zeropool_sharded_storage_common::reconstruct(&values, &storage_config);
        downloaded_data.extend_from_slice(&reconstructed_sector);

        sector_index += 1;
    }

    let decoded_data = decode(&downloaded_data, size);
    fs::write(output, decoded_data)?;

    println!("File downloaded successfully!");
    Ok(())
}
