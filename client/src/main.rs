use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};
use color_eyre::{Report, Result};
use common::{
    config::StorageConfig,
    contract::MockContractClient,
    crypto::derive_keys,
    encode::{decode, encode_aligned},
    node::NodeClient,
};
use p3_matrix::dense::RowMajorMatrix;
use primitives::Val;
use rand::{Rng};
use reqwest::Client;
use tracing_subscriber::fmt::format::FmtSpan;
use client::{download_shards, recover_data};
use shards::{
    compute_commitment, compute_subdomain_indexes,
    recover_original_data_from_subcoset,
};
use common::contract::{ClusterId, UploadClusterReq};
use common::crypto::sign;
use common::node::UploadMessage;

// TODO: Fully libp2p based client
// TODO: tracing

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long)]
    validator_url: String,
    #[arg(short, long)]
    contract_url: String,
}

#[derive(Subcommand)]
enum Commands {
    Upload {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        mnemonic: String,
    },
    Download {
        #[arg(short, long)]
        id: ClusterId,
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::fmt::fmt()
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let client = Client::builder()
        .pool_max_idle_per_host(0)
        .build()?;

    let cli = Cli::parse();
    let validator_client = NodeClient::new(&cli.validator_url, client.clone());
    let contract_client = MockContractClient::new(&cli.contract_url, client.clone());

    match cli.command {
        Commands::Upload { file, mnemonic } => {
            upload_file(file, &mnemonic, &validator_client, &contract_client).await?;
        }
        Commands::Download { id, output } => {
            download_cluster(id, output, &validator_client, client).await?;
        }
    }

    Ok(())
}

async fn upload_file(
    file_path: PathBuf,
    mnemonic: &str,
    validator: &NodeClient,
    contract: &MockContractClient,
) -> Result<()> {
    let t_start = std::time::Instant::now();

    let storage_config = StorageConfig::dev();
    let file_data = fs::read(&file_path)?;

    if file_data.len() > storage_config.cluster_capacity_bytes() {
        return Err(color_eyre::eyre::eyre!("File too large"));
    }

    let serialized_data = bincode::serialize(&file_data)?;
    let encoded_data = encode_aligned(&serialized_data, storage_config.cluster_size())?;
    
    let (private_key, public_key) = derive_keys(mnemonic).unwrap();
    let signature = sign(&encoded_data, private_key);

    let data_matrix = RowMajorMatrix::new(encoded_data.clone(), storage_config.m);
    let (commit, _shards) =
        compute_commitment(data_matrix.clone(), storage_config.log_blowup_factor());

    let t_commitment_end = t_start.elapsed();
    println!("Computed commitment in {t_commitment_end:?}");

    let t_upload_start = std::time::Instant::now();
    // TODO: Ability to overwrite existing clusters
    let cluster_id = contract.reserve_cluster(UploadClusterReq {
        owner_pk: public_key,
        commit: commit.pcs_commitment_hash,
    }).await?;
    
    println!("Uploading file to cluster {}", cluster_id);
    validator
        .upload_cluster(cluster_id.clone(), UploadMessage {
            data: serialized_data,
            signature,
        })
        .await?;

    let t_upload_end = t_upload_start.elapsed();

    println!("Uploaded file in {t_upload_end:?}");
    let t_end = t_start.elapsed();
    println!("Total time: {t_end:?}");

    println!("{cluster_id}");

    Ok(())
}

async fn download_cluster(cluster_id: ClusterId, output: PathBuf, validator: &NodeClient, client: Client) -> Result<()> {
    let storage_config = StorageConfig::dev();
    let nodes = validator.get_info().await?.peers;

    let (shards, subcoset_index) =
        download_shards(cluster_id, &nodes, client).await?;
    let data =
        recover_data(shards, subcoset_index, storage_config.log_blowup_factor())?;

    fs::write(output.clone(), &data)?;


    Ok(())
}
