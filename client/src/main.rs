use clap::{Parser, Subcommand};
use reqwest::multipart;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

// TODO: Proper logging
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Upload {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long, default_value = "8")]
        chunks: usize,
        #[arg(short, long, default_value = "http://localhost:3000")]
        server: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Upload { file, chunks, server } => {
            upload_file(file, *chunks, server).await?;
        }
    }

    Ok(())
}

async fn upload_file(file_path: &PathBuf, chunks: usize, server: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(file_path).await?;
    let file_size = file.metadata().await?.len();
    let chunk_size = (file_size as f64 / chunks as f64).ceil() as usize;

    let client = reqwest::Client::new();

    let nodes_response: Value = client
        .get(&format!("{}/info", server))
        .send()
        .await?
        .json()
        .await?;

    let nodes = nodes_response["peers"].as_array().unwrap();
    if nodes.len() < chunks {
        return Err("Not enough nodes to upload the file".into());
    } else {
        for (i, node) in nodes.iter().enumerate().take(chunks) {
            let start = i * chunk_size;
            let end = if i == chunks - 1 {
                file_size as usize
            } else {
                (i + 1) * chunk_size
            };

            file.seek(std::io::SeekFrom::Start(start as u64)).await?;
            let mut buffer = vec![0u8; end - start];
            file.read_exact(&mut buffer).await?;

            let node_address = node.as_str().unwrap();
            upload_chunk(&client, node_address, file_path, start, end - start).await?;
        }
    }

    println!("File uploaded successfully!");
    Ok(())
}

async fn upload_chunk(
    client: &reqwest::Client,
    server: &str,
    file_path: &PathBuf,
    start: usize,
    length: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_name = file_path.file_name().unwrap().to_str().unwrap();
    let chunk_name = format!("{}_chunk_{}-{}", file_name, start, start + length);

    let mut file = File::open(file_path).await?;
    file.seek(std::io::SeekFrom::Start(start as u64)).await?;
    let mut buffer = vec![0u8; length];
    file.read_exact(&mut buffer).await?;

    let part = multipart::Part::bytes(buffer)
        .file_name(chunk_name.clone())
        .mime_str("application/octet-stream")?;

    let form = multipart::Form::new().part("file", part);

    let response = client
        .post(&format!("{}/upload", server))
        .multipart(form)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Uploaded chunk {} to {}", chunk_name, server);
        Ok(())
    } else {
        Err(format!("Failed to upload chunk {} to {}", chunk_name, server).into())
    }
}