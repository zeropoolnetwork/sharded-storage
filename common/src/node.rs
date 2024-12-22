use std::collections::HashMap;

use color_eyre::Result;
use primitives::Val;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::Instrument;
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
    pub fn new(base_url: &str, client: Client) -> Self {
        NodeClient {
            base_url: base_url.to_string(),
            client,
        }
    }

    #[tracing::instrument(skip(self, msg))]
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

    #[tracing::instrument(skip(self))]
    pub async fn download_cluster(&self, cluster_id: ClusterId) -> Result<Vec<Val>> {
        let url = format!("{}/clusters/{}", self.base_url, cluster_id);

        let span = tracing::info_span!("download_cluster GET", cluster_id = %cluster_id, url = %url);
        let response = self.client.get(&url).send().instrument(span).await?;

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

    #[tracing::instrument(skip(self))]
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
