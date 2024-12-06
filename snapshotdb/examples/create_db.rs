use std::path::PathBuf;
use snapshot_db::db::{SnapshotDb, SnapshotDbConfig};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Create database configuration
    let config = SnapshotDbConfig {
        num_clusters: 1000,    // Number of clusters
        cluster_size: 4096,    // Size of each cluster in bytes
    };

    // Create database path
    let db_path = PathBuf::from("./test_db");
    
    // Create directory if it doesn't exist
    std::fs::create_dir_all(&db_path)?;

    // Initialize the database
    let _db = SnapshotDb::new(db_path, config).await?;
    
    println!("Database created successfully!");

    Ok(())
} 