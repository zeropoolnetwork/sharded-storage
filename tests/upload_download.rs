use std::process::Command;
use tokio::time::{sleep, Duration};

const NUM_NODES: usize = 3;

#[tokio::test]
async fn test_multiple_nodes_info() -> Result<(), Box<dyn std::error::Error>> {
    let mut nodes = vec![];
    for i in 0..NUM_NODES {
        let node = Command::new("cargo")
            .current_dir("..")
            .args(&["run", "--bin", "zeropool-sharded-storage-node", "--", "-a", &format!("0.0.0.0:809{}", i), "-p", &format!("{}", 4001 + i)])
            .spawn()?;
        nodes.push(node);
    }

    // TODO: Better way to wait for nodes to start
    sleep(Duration::from_millis(3000)).await;

    let client = reqwest::Client::new();

    for i in 0..NUM_NODES {
        let url = format!("http://localhost:809{}/info", i);
        let response = client.get(&url).send().await?;
        assert!(response.status().is_success());
        // let info: serde_json::Value = response.json().await?;
        // assert_eq!(info["peers"].as_array().unwrap().len(), 2);
    }

    for mut node in nodes {
        node.kill()?;
    }

    Ok(())
}
