use std::{collections::HashMap, future::Future};
use std::hint::black_box;
use std::sync::Arc;
use client::{download_shards, recover_data};
use common::{config::StorageConfig, node::Peer};
use rand::prelude::SliceRandom;
use tokio::time::{Duration, Instant};
use common::contract::ClusterId;
use primitives::Val;

const CLUSTER_IDS: &[&str] = &[
    "4a09785674d14344d92b1212b6e810369535ea1c",
    "dcfc37347dd5794515d7bb08ffcbca654f47d744",
];
const CONCURRENCY: &[usize] = &[1, 2, 4, 8, 16];
const NUM_REQUESTS: usize = 10;

#[tokio::main(worker_threads = 1000)]
async fn main() {
    let client = common::node::NodeClient::new("http://45.131.67.89:8011");
    let peers = Arc::new(client.get_info().await.unwrap().peers);
    let config = StorageConfig::dev();
    let log_blowup_factor = config.log_blowup_factor();
    
    let bytes_per_request = config.cluster_size() * size_of::<Val>();
    
    for &concurrency in CONCURRENCY {
        let peers = peers.clone();
        let tasks = (0..concurrency).map(move |_| {
            let mut rng = rand::thread_rng();
            let cluster_id: ClusterId = CLUSTER_IDS.choose(&mut rng).unwrap().parse().unwrap();
            let peers = peers.clone();
            tokio::task::spawn(measure_throughput(move || test(peers.clone(), log_blowup_factor, cluster_id.clone()), NUM_REQUESTS))
        });

        let results: Result<Vec<_>, _> = futures::future::join_all(tasks).await.into_iter().collect();
        let (throughputs, avgs): (Vec<f32>, Vec<f32>) = results.unwrap().iter().cloned().unzip();
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
        func().await;
    }

    let elapsed = start.elapsed();
    timings.push(elapsed);

    let throughput = num_requests as f32 / elapsed.as_secs_f32();
    let avg = timings.iter().sum::<Duration>() / num_requests as u32;

    (throughput, avg.as_secs_f32())
}

async fn test(peers: Arc<HashMap<usize, Peer>>, log_blowup_factor: usize, cluster_id: ClusterId) {
    let (shards, subcoset_index) = download_shards(cluster_id, &peers).await.unwrap();
    let data = recover_data(shards, subcoset_index, log_blowup_factor).unwrap();
    black_box(data);
}
