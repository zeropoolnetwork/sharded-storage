use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};
use color_eyre::{Report, Result};
use common::{
    config::StorageConfig,
    contract::MockContractClient,
    crypto::derive_keys,
    encode::{decode, encode, encode_aligned},
    node::NodeClient,
};
use m31jubjub::m31::M31JubJubSigParams;
use p3_matrix::dense::RowMajorMatrix;
use primitives::Val;
use rand::random;
use serde::Serialize;
use shards::{compute_commitment, recover_original_data, recover_original_data_matrix};
use tokio::io::AsyncReadExt;

// TODO: Proper logging
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
        id: u32,
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let validator_client = NodeClient::new(&cli.validator_url);
    let contract_client = MockContractClient::new(&cli.contract_url);

    match cli.command {
        Commands::Upload { file, mnemonic } => {
            upload_file(file, &mnemonic, &validator_client, &contract_client).await?;
        }
        Commands::Download { id, output } => {
            download_cluster(id, output, &validator_client).await?;
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
    let config = StorageConfig::dev();
    let file_data = fs::read(&file_path)?;

    if file_data.len() > config.cluster_size_bytes() {
        return Err(color_eyre::eyre::eyre!("File too large"));
    }

    let serialized_data = bincode::serialize(&file_data)?;
    let encoded_data = encode_aligned(&serialized_data, config.m)?
        .into_iter()
        .chain((0..).map(|_| Val::new(0)))
        .take(config.cluster_size())
        .collect::<Vec<_>>();

    let sig_params = M31JubJubSigParams::default();
    let (private_key, public_key) = derive_keys(mnemonic).unwrap();

    let data_matrix = RowMajorMatrix::new(encoded_data.clone(), config.m);
    let (commit, _shards) = compute_commitment(data_matrix, config.log_blowup_factor());

    let cluster_id: u32 = random();

    println!("Uploading file to cluster {}", cluster_id);
    validator.upload_cluster(cluster_id, encoded_data).await?;

    println!("File uploaded successfully!");
    Ok(())
}

async fn upload_cluster(index: usize, validator_url: &str, data: Vec<Val>) -> Result<()> {
    let validator = NodeClient::new(validator_url);
    validator.upload_cluster(index as u32, data).await?;

    Ok(())
}

async fn download_cluster(cluster_id: u32, output: PathBuf, validator: &NodeClient) -> Result<()> {
    let t_full_start = std::time::Instant::now();

    let storage_config = StorageConfig::dev();
    let nodes = validator.get_info().await?.peers;
    let num_shards = nodes.len();

    let t_shards_start = std::time::Instant::now();

    let futures = nodes
        .into_iter()
        .take(storage_config.m)
        .map(|(node_id, url)| {
            tokio::spawn(async move {
                let client = NodeClient::new(&url);
                let data = client.download_cluster(cluster_id).await?;
                Ok::<_, Report>((node_id, data))
            })
        })
        .collect::<Vec<_>>();

    let mut shards: Vec<(usize, Vec<Val>)> = Vec::with_capacity(num_shards);
    for future in futures {
        shards.push(future.await??);
    }

    let t_shards_end = t_shards_start.elapsed();
    println!("Downloaded {num_shards} shards in {t_shards_end:?}");

    let t_shards_recovery_start = std::time::Instant::now();

    let (indices, shards): (Vec<usize>, Vec<Vec<Val>>) = shards.into_iter().unzip();
    let elements = shards.into_iter().flatten().collect::<Vec<_>>();
    let shards_data = RowMajorMatrix::new(elements, storage_config.m);

    let log_dimension = storage_config.n.ilog2() as usize;
    let recover_matrix =
        recover_original_data_matrix(log_dimension, &indices, storage_config.log_blowup_factor());
    let recovered_data = recover_original_data(shards_data, &recover_matrix);

    let decoded_data = decode(&recovered_data.values, storage_config.m);
    let reader = std::io::Cursor::new(decoded_data);
    let deserialized_data: Vec<u8> = bincode::deserialize_from(reader)?;

    fs::write(output, &deserialized_data)?;

    let t_shards_recovery_end = t_shards_recovery_start.elapsed();
    println!("Recovered original data in {t_shards_recovery_end:?}");
    let t_full_end = t_full_start.elapsed();
    println!("Downloaded file in {t_full_end:?}");
    Ok(())
}
