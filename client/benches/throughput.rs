use std::{collections::HashMap, future::Future};
use std::hint::black_box;
use std::sync::Arc;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use client::{download_shards, recover_data};
use common::{config::StorageConfig, node::Peer};
use rand::prelude::SliceRandom;
use reqwest::Client;
use tokio::time::{Duration, Instant};
use tracing_subscriber::fmt::format::FmtSpan;
use common::contract::ClusterId;
use common::node::NodeClient;
use primitives::Val;

const CLUSTER_IDS: &[&str] = &[
    "4a09785674d14344d92b1212b6e810369535ea1c",
    "dcfc37347dd5794515d7bb08ffcbca654f47d744",
    "dbf5013b65b95c339ecd6563acd4b8016cd0d80f",
    "486818732c691850ddcd5b241ca23319454fe575",
    "4bb7275205086e01c4bdef60113abd1c6c07b666",
    "d16bf0750fcda12088c406510f2d2f6c50d4097c",
    "f4468b46760db96b07658c71338db961fb6de72f",
    "6bfb0a0bfd71b41f71bd956b5e6af76c8ad5cd2b",
    "4a7fb852bb3f120e676f906c7e208e43f6dc1003",
    "ef4d97771d424720fb370d0e82f4537efb72c47a",
    "b8186e0e1806966514ea8d45b3eb3e7681bdf974",
    "c544b2178af4a4428cd1e12ca26d6428e3d24276",
];
const CONCURRENCY: &[usize] = &[1, 4, 8, 16, 32];
const NUM_REQUESTS: usize = 10;

const VALIDATOR_URL: &str = "http://45.131.67.89:8011";

#[tokio::main]
async fn main() {
    // tracing_subscriber::fmt::fmt()
    //     .with_span_events(FmtSpan::CLOSE)
    //     .init();

    let client = Client::builder()
        // .pool_max_idle_per_host(0)
        .build()
        .unwrap();
    let validator = NodeClient::new(VALIDATOR_URL, client.clone());
    let peers = Arc::new(validator.get_info().await.unwrap().peers);
    let config = StorageConfig::dev();
    let log_blowup_factor = config.log_blowup_factor();
    
    let bytes_per_request = config.cluster_size() * size_of::<Val>();
    
    for &concurrency in CONCURRENCY {
        let peers = peers.clone();
        let mut tasks = FuturesUnordered::new();
        for client_index in 0..concurrency {
            let peers = peers.clone();
            let mut rng = rand::thread_rng();
            let cluster_id: ClusterId = CLUSTER_IDS.choose(&mut rng).unwrap().parse().unwrap();
            let client = client.clone();
            tasks.push(measure_throughput(move || test(peers.clone(), log_blowup_factor, cluster_id.clone(), client.clone()), NUM_REQUESTS));
        }

        let mut results = Vec::new();
        while let Some(result) = tasks.next().await {
            results.push(result);
        }

        let (throughputs, avgs): (Vec<f32>, Vec<f32>) = results.into_iter().unzip();
        let total_throughput = throughputs.iter().sum::<f32>() as f32;
        let avg = avgs.iter().sum::<f32>() / concurrency as f32;
        let total_bytes = total_throughput * bytes_per_request as f32;
        let total_megabytes = total_bytes / 1024.0 / 1024.0;

        println!(
            "concurrency: {}, total throughput: {:.2} MB/sec, avg request time: {:.2} sec",
            concurrency, total_megabytes, avg,
        );
        
    }
    
}

async fn measure_throughput<F, Fut>(func: F, num_requests: usize) -> (f32, f32)
where
    F: Fn() -> Fut,
    Fut: Future<Output = ()>,
{
    let mut timings = Vec::with_capacity(num_requests);
    let start = Instant::now();

    for _ in 0..num_requests {
        let start = Instant::now();
        func().await;
        timings.push(start.elapsed());
    }

    let total_time = start.elapsed();

    let throughput = num_requests as f32 / total_time.as_secs_f32();
    let avg = timings.iter().sum::<Duration>() / num_requests as u32;

    (throughput, avg.as_secs_f32())
}

async fn test(peers: Arc<HashMap<usize, Peer>>, log_blowup_factor: usize, cluster_id: ClusterId, client: Client) {
    let (shards, subcoset_index) = download_shards(cluster_id, &peers, client).await.unwrap();
    let data = recover_data(shards, subcoset_index, log_blowup_factor).unwrap();
    black_box(data);
}
