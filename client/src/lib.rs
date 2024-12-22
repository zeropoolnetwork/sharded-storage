use std::collections::HashMap;

use color_eyre::{Report, Result};
use common::{
    config::StorageConfig,
    contract::{ClusterId, MockContractClient, UploadClusterReq},
    crypto::{derive_keys, sign},
    encode::{decode, encode_aligned},
    node::{NodeClient, Peer, UploadMessage},
};
use futures::{stream::FuturesUnordered, StreamExt};
use p3_matrix::dense::RowMajorMatrix;
use primitives::Val;
use rand::{thread_rng, Rng};
use reqwest::Client;
use shards::{compute_commitment, compute_subdomain_indexes, recover_original_data_from_subcoset};
use tracing::instrument;

pub async fn upload_cluster(
    data: Vec<u8>,
    mnemonic: &str,
    validator: &NodeClient,
    contract: &MockContractClient,
) -> Result<()> {
    let t_start = std::time::Instant::now();

    let storage_config = StorageConfig::dev();

    if data.len() > storage_config.cluster_capacity_bytes() {
        return Err(color_eyre::eyre::eyre!("File too large"));
    }

    let serialized_data = bincode::serialize(&data)?;
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
    let cluster_id = contract
        .reserve_cluster(UploadClusterReq {
            owner_pk: public_key,
            commit: commit.pcs_commitment_hash,
        })
        .await?;

    println!("Uploading file to cluster {}", cluster_id);
    validator
        .upload_cluster(
            cluster_id,
            UploadMessage {
                data: serialized_data,
                signature,
            },
        )
        .await?;

    let t_upload_end = t_upload_start.elapsed();

    println!("Uploaded file in {t_upload_end:?}");
    let t_end = t_start.elapsed();
    println!("Total time: {t_end:?}");

    Ok(())
}

#[instrument(skip(nodes))]
pub async fn download_shards(
    cluster_id: ClusterId,
    nodes: &HashMap<usize, Peer>,
    client: Client,
) -> Result<(Vec<Vec<Val>>, usize)> {
    let storage_config = StorageConfig::dev();

    let log_blowup_factor = storage_config.log_blowup_factor();
    let subcoset_index = thread_rng().gen_range(0..(1 << log_blowup_factor));
    let subcoset_indices = compute_subdomain_indexes(
        subcoset_index,
        log_blowup_factor,
        storage_config.m.ilog2() as usize,
    );

    let num_shards = subcoset_indices.len();
    let mut tasks = FuturesUnordered::new();
    for node_id in subcoset_indices.into_iter().take(storage_config.m) {
        let node = nodes[&node_id].clone();
        let cluster_id = cluster_id.clone();
        let client = client.clone();
        tasks.push(async move {
            let node_client = NodeClient::new(&node.api_url, client.clone());
            let data = node_client.download_cluster(cluster_id.clone()).await?;
            Ok::<_, Report>((node_id, data))
        })
    }

    let mut shards = Vec::with_capacity(num_shards);
    while let Some(result) = tasks.next().await {
        shards.push(result?);
    }

    shards.sort_by_key(|(node_id, _)| *node_id);
    let shards = shards.into_iter().map(|(_, data)| data).collect();

    Ok((shards, subcoset_index))
}

#[instrument(skip_all)]
pub fn recover_data(
    shards: Vec<Vec<Val>>,
    subcoset_index: usize,
    log_blowup_factor: usize,
) -> Result<Vec<u8>> {
    let storage_config = StorageConfig::dev();

    let subcoset_data =
        RowMajorMatrix::new(shards.into_iter().flatten().collect(), storage_config.n);

    let recovered_data =
        recover_original_data_from_subcoset(subcoset_data, subcoset_index, log_blowup_factor);

    let decoded_data = decode(
        &recovered_data.values,
        storage_config.cluster_capacity_bytes(),
    );

    let reader = std::io::Cursor::new(decoded_data);
    let deserialized_data: Vec<u8> = bincode::deserialize_from(reader)?;

    Ok(deserialized_data)
}
