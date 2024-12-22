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
const CONCURRENCY: &[usize] = &[1, 2, 4, 8, 16];
const NUM_REQUESTS: usize = 10;

#[tokio::main]
async fn main() {
    let client = common::node::NodeClient::new("http://45.131.67.89:8011");
    let peers = Arc::new(client.get_info().await.unwrap().peers);
    let config = StorageConfig::dev();
    let log_blowup_factor = config.log_blowup_factor();
    
    let bytes_per_request = config.cluster_size() * size_of::<Val>();
    
    for &concurrency in CONCURRENCY {
        let peers = peers.clone();
        let mut tasks = Vec::new();
        for _ in 0..concurrency {
            let peers = peers.clone();
            let mut rng = rand::thread_rng();
            let cluster_id: ClusterId = CLUSTER_IDS.choose(&mut rng).unwrap().parse().unwrap();

            tasks.push(std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                runtime.block_on(async move {
                    measure_throughput(move || test(peers.clone(), log_blowup_factor, cluster_id.clone()), NUM_REQUESTS).await
                })
            }));
        }

        let mut results = Vec::new();
        for handle in tasks {
            results.push(handle.join().unwrap());
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
