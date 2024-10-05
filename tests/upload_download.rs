use std::process::{Child, Command};

use tokio::time::{sleep, Duration};

const NUM_NODES: usize = 3;

#[tokio::test]
async fn test_multiple_nodes_info() -> Result<(), Box<dyn std::error::Error>> {
    let mut nodes = vec![];
    for i in 0..NUM_NODES {
        let node = run_node(&format!("0.0.0.0:809{}", i), 4001 + i as u16)?;
        nodes.push(node);
    }

    // TODO: Better way to wait for nodes to start
    sleep(Duration::from_millis(3000)).await;

    let client = reqwest::Client::new();

    for i in 0..NUM_NODES {
        assert!(is_node_running(&format!("localhost:809{}", i)).await);
    }

    run_upload()?;
    run_download()?;

    for mut node in nodes {
        node.kill()?;
    }

    Ok(())
}

fn run_node(addr: &str, port: u16) -> Result<Child, Box<dyn std::error::Error>> {
    let child = Command::new("cargo")
        .current_dir("..")
        .args(&[
            "run",
            "--bin",
            "zeropool-sharded-storage-node",
            "--",
            "-a",
            addr,
            "-p",
            &format!("{}", port),
        ])
        .spawn()?;

    Ok(child)
}

async fn is_node_running(addr: &str) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://{}/info", addr);
    let response = client.get(&url).send().await;
    response.is_ok()
}

fn run_upload() -> Result<Child, Box<dyn std::error::Error>> {
    let child = Command::new("cargo")
        .current_dir("..")
        .args(&[
            "run",
            "--bin",
            "zeropool-storage-client",
            "--",
            "upload",
            "--file",
            "test.txt",
        ])
        .spawn()?;

    Ok(child)
}

fn run_download() -> Result<Child, Box<dyn std::error::Error>> {
    let child = Command::new("cargo")
        .current_dir("..")
        .args(&[
            "run",
            "--bin",
            "zeropool-storage-client",
            "--",
            "download",
            "--id",
            "1",
            "--output",
            "test.txt",
            "--size",
            "100",
        ])
        .spawn()?;

    Ok(child)
}
