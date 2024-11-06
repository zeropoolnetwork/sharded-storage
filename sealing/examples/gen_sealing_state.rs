use primitives::{LOG_SEGMENT_SIZE, LOG_VOLUME_SIZE, LOG_FRAGMENT_SIZE, NUM_NODES, POSEIDON2_HASH, Hash};
use itertools::{iproduct, Itertools};
use p3_maybe_rayon::prelude::*;
use p3_field::Field;
use p3_symmetric::CryptographicHasher;
use std::fs::OpenOptions;
use std::io::Write;
use bincode;
use indicatif::{ProgressBar, ProgressStyle};

use sealing::{get_fragment_seed, sealing_vec};

const NUM_VOLUMES: usize = 1;

/// Checks if a number is a power of two
fn is_power_of_two(n: usize) -> bool {
    n != 0 && (n & (n - 1)) == 0
}

/// Computes the Merkle root of a vector of field elements.
/// Values are grouped into leaves of size 8, then a Merkle tree is built on top of them.
/// 
/// # Type Parameters
/// * `F` - Field element type
/// * `H` - Hasher implementation
/// * `DIGEST_ELEMS` - Size of hash output array
///
/// # Arguments
/// * `hasher` - The hash function implementation
/// * `values` - Input vector of field elements
fn compute_merkle_root<F, H, const DIGEST_ELEMS: usize>(
    hasher: &H, 
    values: &[F],
) -> [F; DIGEST_ELEMS] 
where
    F: Field,
    H: CryptographicHasher<F, [F; DIGEST_ELEMS]>,
{
    assert!(is_power_of_two(values.len()));

    const LEAF_SIZE: usize = 8;
    
    // Create first layer without recursion
    let mut current_layer: Vec<[F; DIGEST_ELEMS]> = Vec::with_capacity(values.len() / LEAF_SIZE);
    
    // Process input data in LEAF_SIZE chunks
    for chunk in values.chunks(LEAF_SIZE) {
        let mut leaf = vec![F::zero(); LEAF_SIZE];
        leaf[..chunk.len()].copy_from_slice(chunk);
        current_layer.push(hasher.hash_iter(leaf));
    }

    // Iteratively build Merkle tree
    while current_layer.len() > 1 {
        let mut next_layer = Vec::with_capacity(current_layer.len() / 2);
        
        for chunk in current_layer.chunks(2) {
            let hash_input: Vec<F> = chunk.iter()
                .flat_map(|x| x.iter())
                .copied()
                .collect();
            next_layer.push(hasher.hash_iter(hash_input));
        }
        
        current_layer = next_layer;
    }

    current_layer[0]
}

fn main() {
    const NUM_SEGMENTS_IN_VOLUME: usize = 1 << (LOG_VOLUME_SIZE - LOG_SEGMENT_SIZE);
    const NUM_FRAGMENTS_IN_SEGMENT: usize = 1 << (LOG_SEGMENT_SIZE - LOG_FRAGMENT_SIZE);
    const CHUNK_SIZE: usize = 128;

    // Calculate total number of iterations for progress bar
    let total_iterations = NUM_NODES * NUM_VOLUMES * NUM_SEGMENTS_IN_VOLUME * NUM_FRAGMENTS_IN_SEGMENT;
    
    // Initialize progress bar
    let pb = ProgressBar::new(total_iterations as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({percent}%) \n➤ Chunk: {msg}\n➤ Speed: {per_sec:.green} items/sec\n➤ ETA: {eta_precise}\n➤ Total time estimate: {duration_precise}")
        .expect("Failed to set progress bar style")
        .progress_chars("##-"));
    std::io::stdout().flush().unwrap();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("hashes.bin")
        .expect("Failed to create hashes.bin");

    let chunks = iproduct!(
        0..NUM_NODES, 
        0..NUM_VOLUMES, 
        0..NUM_SEGMENTS_IN_VOLUME, 
        0..NUM_FRAGMENTS_IN_SEGMENT
    )
    .chunks(CHUNK_SIZE);

    let total_chunks = (total_iterations + CHUNK_SIZE - 1) / CHUNK_SIZE;
    let mut processed_chunks = 0;

    chunks.into_iter().for_each(|chunk| {
        let chunk = chunk.collect_vec();
        let hashes: Vec<Hash> = chunk.into_par_iter()
            .map(|(node_id, volume_id, segment_id, fragment_id)| {
                let seed = get_fragment_seed(node_id, volume_id, segment_id, fragment_id);
                let data = sealing_vec(seed);
                let root = compute_merkle_root(&*POSEIDON2_HASH, &data);
                root.into()
            })
            .collect();

        let serialized = bincode::serialize(&hashes)
            .expect("Failed to serialize hashes");
        file.write_all(&serialized[8..])
            .expect("Failed to write hashes to file");

        // Update progress
        processed_chunks += 1;
        pb.set_position((processed_chunks * CHUNK_SIZE).min(total_iterations) as u64);
        pb.set_message(format!("Chunk {}/{}", processed_chunks, total_chunks));
    });

    // Finish progress bar and show final statistics
    pb.finish_with_message(format!("Completed {} chunks! Total time: {}", total_chunks, 
        humantime::format_duration(pb.elapsed()).to_string()));
}