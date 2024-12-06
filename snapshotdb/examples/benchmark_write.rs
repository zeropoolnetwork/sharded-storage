use std::time::Instant;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;
use snapshot_db::db::{SnapshotDb, SnapshotDbConfig};

const CLUSTER_SIZE: usize = 1024 * 1024; // 1 MB
const NUM_CLUSTERS: usize = 1024;
const NUM_THREADS: usize = 12;

async fn run_single_threaded_benchmark(db: Arc<SnapshotDb>, test_data: &[u8]) {
    println!("Starting single-threaded write test...");
    let start = Instant::now();

    for i in 0..NUM_CLUSTERS {
        db.write(i, test_data).await.unwrap();
    }

    let duration = start.elapsed();
    let throughput = (NUM_CLUSTERS * CLUSTER_SIZE) as f64 / duration.as_secs_f64() / (1024.0 * 1024.0);
    println!("Single-threaded write: {:.2} MB/s", throughput);
}

async fn run_multi_threaded_benchmark(db: Arc<SnapshotDb>, test_data: &[u8]) {
    println!("\nStarting {}-threaded write test...", NUM_THREADS);
    let start = Instant::now();
    let mut set = JoinSet::new();

    for i in 0..NUM_CLUSTERS {
        let db = Arc::clone(&db);
        let test_data = test_data.to_vec();
        set.spawn(async move {
            db.write(i, &test_data).await.unwrap();
        });

        // Limit concurrent tasks
        if set.len() >= NUM_THREADS {
            set.join_next().await.unwrap().unwrap();
        }
    }

    // Wait for remaining tasks
    while let Some(result) = set.join_next().await {
        result.unwrap();
    }

    let duration = start.elapsed();
    let throughput = (NUM_CLUSTERS * CLUSTER_SIZE) as f64 / duration.as_secs_f64() / (1024.0 * 1024.0);
    println!("{}-threaded write: {:.2} MB/s", NUM_THREADS, throughput);
}

#[tokio::main]
async fn main() {
    let path = PathBuf::from("benchmark");
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&path).unwrap();

    let config = SnapshotDbConfig {
        cluster_size: CLUSTER_SIZE,
        num_clusters: NUM_CLUSTERS,
    };

    let db = Arc::new(SnapshotDb::new(&path, config).await.unwrap());
    
    // Create test data
    let test_data = vec![42u8; CLUSTER_SIZE];

    // Run benchmarks
    run_single_threaded_benchmark(Arc::clone(&db), &test_data).await;
    run_multi_threaded_benchmark(Arc::clone(&db), &test_data).await;
}