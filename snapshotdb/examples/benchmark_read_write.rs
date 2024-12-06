use std::time::Instant;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinSet;
use snapshot_db::db::{SnapshotDb, SnapshotDbConfig};

const CLUSTER_SIZE: usize = 1024 * 1024; // 1 MB
const NUM_CLUSTERS: usize = 1024;
const WRITE_THREADS: usize = 12;
const READ_THREADS: usize = 24;

async fn run_concurrent_read_write_benchmark(db: Arc<SnapshotDb>, test_data: &[u8]) {
    println!("\nStarting concurrent read/write benchmark...");
    println!("Write threads: {}", WRITE_THREADS);
    println!("Read threads: {}", READ_THREADS);
    
    let start = Instant::now();
    let mut write_set = JoinSet::new();
    let mut read_set = JoinSet::new();

    // Launch write tasks
    for i in 0..NUM_CLUSTERS {
        let test_data = test_data.to_vec();
        let cloned_db = db.clone();
        write_set.spawn(async move {
            cloned_db.write(i, &test_data).await.unwrap();
        });

        // Limit the number of concurrent write tasks
        if write_set.len() >= WRITE_THREADS {
            write_set.join_next().await.unwrap().unwrap();
        }

        for _ in 0..2 { // Launch 2 reads for each write
            let cloned_db = db.clone();
            let read_index = fastrand::usize(..NUM_CLUSTERS); // Random index from already written
            read_set.spawn(async move {
                let _data = cloned_db.read(0, read_index).await.unwrap();
            });

            // Limit the number of concurrent read tasks
            if read_set.len() >= READ_THREADS {
                read_set.join_next().await.unwrap().unwrap();
            }
        }
        
    }

    // Wait for remaining write tasks to complete
    while let Some(result) = write_set.join_next().await {
        result.unwrap();
    }

    // Wait for remaining read tasks to complete
    while let Some(result) = read_set.join_next().await {
        result.unwrap();
    }

    let duration = start.elapsed();
    
    // Calculate total volume of written data
    let write_volume = NUM_CLUSTERS * CLUSTER_SIZE;
    let write_throughput = write_volume as f64 / duration.as_secs_f64() / (1024.0 * 1024.0);
    
    // Calculate total volume of read data (approximately 2 reads for each write)
    let read_volume = write_volume * 2;
    let read_throughput = read_volume as f64 / duration.as_secs_f64() / (1024.0 * 1024.0);

    println!("\nResults after {:.2} seconds:", duration.as_secs_f64());
    println!("Write throughput: {:.2} MB/s", write_throughput);
    println!("Read throughput: {:.2} MB/s", read_throughput);
    println!("Combined throughput: {:.2} MB/s", write_throughput + read_throughput);
}

#[tokio::main]
async fn main() {
    let path = PathBuf::from("benchmark_rw");
    
    // Create directory if it does not exist
    std::fs::create_dir_all(&path).unwrap();

    let config = SnapshotDbConfig {
        cluster_size: CLUSTER_SIZE,
        num_clusters: NUM_CLUSTERS,
    };

    let db = Arc::new(SnapshotDb::new(&path, config).await.unwrap());
    
    // Create test data
    let test_data = vec![42u8; CLUSTER_SIZE];

    // Run benchmark
    run_concurrent_read_write_benchmark(Arc::clone(&db), &test_data).await;
}