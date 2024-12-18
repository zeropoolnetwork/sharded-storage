use color_eyre::Result;
use primitives::Val;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub peers: Vec<String>,
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

    pub async fn upload_cluster(&self, cluster_id: u32, elements: Vec<Val>) -> Result<()> {
        let url = format!("{}cluster/{}", self.base_url, cluster_id);
        let data = bincode::serialize(&elements)?;
        let form =
            reqwest::multipart::Form::new().part("file", reqwest::multipart::Part::bytes(data));

        let response = self.client.post(&url).multipart(form).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(color_eyre::eyre::eyre!("Failed to upload cluster"))
        }
    }

    pub async fn download_cluster(&self, cluster_id: u32) -> Result<Vec<Val>> {
        let url = format!("{}cluster/{}", self.base_url, cluster_id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let data = response.bytes().await?;
            let elements: Vec<Val> = bincode::deserialize(&data)?;
            Ok(elements)
        } else {
            Err(color_eyre::eyre::eyre!("Failed to download cluster"))
        }
    }

    pub async fn get_info(&self) -> Result<InfoResponse> {
        let url = format!("{}info", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let info: InfoResponse = response.json().await?;
            Ok(info)
        } else {
            Err(color_eyre::eyre::eyre!("Failed to get info"))
        }
    }
}
