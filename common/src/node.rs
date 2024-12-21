use std::collections::HashMap;

use color_eyre::Result;
use primitives::Val;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::contract::ClusterId;
use crate::crypto::Signature;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Peer {
    pub peer_id: String,
    pub addr: String,
    pub api_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub peers: HashMap<usize, Peer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadMessage {
    /// Original, unencoded data
    pub data: Vec<u8>,
    pub signature: Signature,
}

#[derive(Debug)]
pub struct NodeClient {
    base_url: String,
    client: Client,
}

impl NodeClient {
    pub fn new(base_url: &str) -> Self {
        NodeClient {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn upload_cluster(&self, cluster_id: ClusterId, msg: UploadMessage) -> Result<()> {
        let url = format!("{}/clusters/{}", self.base_url, cluster_id);
        let data = bincode::serialize(&msg)?;
        let form =
            reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(data));

        let response = self.client.post(&url).multipart(form).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(color_eyre::eyre::eyre!("Failed to upload cluster"))
        }
    }

    pub async fn download_cluster(&self, cluster_id: ClusterId) -> Result<Vec<Val>> {
        let url = format!("{}/clusters/{}", self.base_url, cluster_id);
        let t_start = std::time::Instant::now();
        let response = self.client.get(&url).send().await?;
        let t_end = t_start.elapsed();
        // println!("    Response time {t_end:?}");

        if response.status().is_success() {
            let data = response.bytes().await?.to_vec();
            let elements: Vec<Val> = unsafe {
                data[..].align_to::<Val>().1.to_vec()
            };
            Ok(elements)
        } else {
            Err(color_eyre::eyre::eyre!("Failed to download cluster"))
        }
    }

    pub async fn get_info(&self) -> Result<InfoResponse> {
        let url = format!("{}/info", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let info: InfoResponse = response.json().await?;
            Ok(info)
        } else {
            Err(color_eyre::eyre::eyre!("Failed to get info"))
        }
    }
}
