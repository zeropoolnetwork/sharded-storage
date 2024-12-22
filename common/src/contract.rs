use std::str::FromStr;
use color_eyre::Result;
use primitives::{Hash, Val};
use rand::random;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::crypto::PublicKey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub index: u64,
    pub owner_pk: PublicKey,
    pub commit: Hash,
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ClusterId(pub [Val; 5]);

impl ClusterId {
    pub fn random() -> Self {
        ClusterId(random())
    }
}

impl FromStr for ClusterId {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut bytes = [0u8; 20];
        hex::decode_to_slice(s, &mut bytes)?;
        Ok(bytes.into())
    }
}

impl std::fmt::Debug for ClusterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes: &[u8; 20] = self.as_ref();
        write!(f, "{}", hex::encode(bytes))
    }
}

impl std::fmt::Display for ClusterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes: &[u8; 20] = self.as_ref();
        write!(f, "{}", hex::encode(bytes))
    }
}

impl AsRef<[u8]> for ClusterId {
    fn as_ref(&self) -> &[u8] {
        &<ClusterId as AsRef<[u8; 20]>>::as_ref(self)[..]
    }
}

impl AsRef<[u8; 20]> for ClusterId {
    fn as_ref(&self) -> &[u8; 20] {
        static_assertions::assert_eq_size!(ClusterId, [u8; 20]);
        // Safety: enforced by the assertion above
        unsafe { std::mem::transmute(&self.0) }
    }
}

impl From<[u8; 20]> for ClusterId {
    fn from(bytes: [u8; 20]) -> Self {
        static_assertions::assert_eq_size!(ClusterId, [u8; 20]);
        // Safety: enforced by the assertion above
        ClusterId(unsafe { std::mem::transmute(bytes) })
    }
}

#[derive(Debug, Clone)]
pub struct MockContractClient {
    base_url: String,
    client: Client,
}

#[derive(Serialize, Deserialize)]
pub struct UploadClusterReq {
    pub owner_pk: PublicKey,
    pub commit: Hash,
}

impl MockContractClient {
    pub fn new(url: &str) -> Self {
        MockContractClient {
            base_url: url.to_string(),
            client: Client::new(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_info(&self) -> Result<serde_json::Value> {
        let url = format!("{}/info", self.base_url);
        let response = self.client.get(&url).send().await?;
        response.json().await.map_err(Into::into)
    }

    #[tracing::instrument(skip(self, cluster))]
    pub async fn reserve_cluster(&self, cluster: UploadClusterReq) -> Result<ClusterId> {
        #[derive(Deserialize)]
        struct UploadClusterRes {
            cluster_id: String,
        }

        let url = format!("{}/clusters", self.base_url);

        let response: UploadClusterRes = self
            .client
            .post(&url)
            .json(&cluster)
            .send()
            .await?
            .json()
            .await?;

        Ok(response.cluster_id.parse()?)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_cluster(&self, cluster_id: &ClusterId) -> Result<Cluster> {
        let url = format!("{}/clusters/{}", self.base_url, cluster_id);
        let response = self.client.get(&url).send().await?;
        Ok(response.json().await?)
    }
}
