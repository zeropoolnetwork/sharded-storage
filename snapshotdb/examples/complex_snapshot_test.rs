use std::path::PathBuf;
use snapshot_db::db::{SnapshotDb, SnapshotDbConfig};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

const CLUSTER_SIZE: usize = 4096;
const NUM_CLUSTERS: usize = 100;
const NUM_SNAPSHOTS: usize = 5;
const MASTER_SEED: u64 = 42;

/// Generates deterministic pseudo-random data for a cluster based on cluster_id and snapshot_id
fn generate_cluster_data(snapshot_id: usize, cluster_id: usize) -> Vec<u8> {
    // Create a unique seed for each (snapshot_id, cluster_id) pair
    let seed = MASTER_SEED
        .wrapping_mul(snapshot_id as u64)
        .wrapping_add(cluster_id as u64);
    
    let mut rng = StdRng::seed_from_u64(seed);
    let mut data = vec![0u8; CLUSTER_SIZE];
    for chunk in data.chunks_mut(8) {
        let val = rng.gen::<u64>();
        chunk.copy_from_slice(&val.to_le_bytes());
    }
    data
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("Starting complex snapshot test...");

    // Create database configuration
    let config = SnapshotDbConfig {
        num_clusters: NUM_CLUSTERS,
        cluster_size: CLUSTER_SIZE,
    };

    // Clean up any existing test databases
    let test_dir = PathBuf::from("./test_complex_db");
    let parent_dir = test_dir.parent().unwrap_or(&test_dir);
    for entry in std::fs::read_dir(parent_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.file_name().unwrap().to_string_lossy().starts_with("test_complex_db") {
            println!("Cleaning up old test database at {:?}", path);
            std::fs::remove_dir_all(path)?;
        }
    }

    // Create database directory
    std::fs::create_dir_all(&test_dir)?;

    // Initialize the database
    let db = SnapshotDb::new(&test_dir, config).await?;
    println!("Database created successfully!");

    let mut rng = StdRng::seed_from_u64(MASTER_SEED);

    // Snapshots (initial) and 1 (pending) are created by default
    println!("\nCreating initial snapshot and writing data...");

    let mut snapshot_data: Vec<Vec<Vec<u8>>> = vec![vec![vec![0u8; CLUSTER_SIZE]; NUM_CLUSTERS]];
    snapshot_data.push(snapshot_data.last().unwrap().clone());

    for cluster_id in 0..NUM_CLUSTERS {
        let data = generate_cluster_data(1, cluster_id);
        db.write(cluster_id, &data).await?;
        snapshot_data[1][cluster_id] = data;
    }

    // Create additional snapshots with modifications
    for snapshot_id in 2..=NUM_SNAPSHOTS {
        println!("\nCreating snapshot {}", snapshot_id);
        db.add_snapshot().await?;
        snapshot_data.push(snapshot_data.last().unwrap().clone());

        // Modify random clusters in this snapshot
        let num_modifications = rng.gen_range(5..15);
        println!("Making {} modifications in snapshot {}", num_modifications, snapshot_id);

        for _ in 0..num_modifications {
            let cluster_id = rng.gen_range(0..NUM_CLUSTERS);
            let data = generate_cluster_data(snapshot_id, cluster_id);
            db.write(cluster_id, &data).await?;
            snapshot_data[snapshot_id][cluster_id] = data;
        }
    }

    // Verify data in all snapshots
    println!("\nVerifying data in all snapshots...");
    for snapshot_id in 1..=NUM_SNAPSHOTS {
        for cluster_id in 0..NUM_CLUSTERS {
            let expected = snapshot_data[snapshot_id][cluster_id].clone();
            let actual = db.read(snapshot_id, cluster_id).await?;
            assert_eq!(actual, expected, 
                "Data mismatch in snapshot {} cluster {}", 
                snapshot_id, cluster_id);
        }
        println!("✓ Snapshot {} verified successfully", snapshot_id);
    }

    // Test snapshot joining
    println!("\nTesting snapshot joining...");
    db.join_snapshot().await?;
    println!("Joined oldest snapshot");

    // Verify remaining snapshots after join
    for snapshot_id in 2..=NUM_SNAPSHOTS {
        for cluster_id in 0..NUM_CLUSTERS {
            let expected = snapshot_data[snapshot_id][cluster_id].clone();
            let actual = db.read(snapshot_id, cluster_id).await?;
            assert_eq!(actual, expected, 
                "Data mismatch after join in snapshot {} cluster {}", 
                snapshot_id, cluster_id);
        }
        println!("✓ Snapshot {} verified successfully after join", snapshot_id);
    }

    println!("\nAll tests passed successfully!");
    Ok(())
} 