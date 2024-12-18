use std::net::SocketAddr;

use color_eyre::Result;
use primitives::{Hash, Val};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::crypto::PublicKey;

type LogicalSegmentId = Vec<Val>;
type SlotId = u64;
type SegmentId = [Val; 8];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub owner_pk: PublicKey,
    pub segments: Vec<SegmentId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub slot: SlotId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadSegmentReq {
    pub segment: SegmentId,
    pub slot: SlotId,
    pub owner_pk: PublicKey,
    pub commit: Hash,
}

#[derive(Debug, Clone)]
pub struct MockContractClient {
    pub base_url: String,
    pub client: Client,
}

impl MockContractClient {
    pub fn new(url: &str) -> Self {
        MockContractClient {
            base_url: url.to_string(),
            client: Client::new(),
        }
    }

    pub async fn get_info(&self) -> Result<serde_json::Value> {
        let url = format!("{}/info", self.base_url);
        let response = self.client.get(&url).send().await?;
        response.json().await.map_err(Into::into)
    }

    pub async fn get_slot_segments(&self, slot_id: SlotId) -> Result<Vec<SegmentId>> {
        let url = format!("{}/slots/{}/segments", self.base_url, slot_id);
        let response = self.client.get(&url).send().await?;
        response.json().await.map_err(Into::into)
    }

    pub async fn upload_segment(&self, req: UploadSegmentReq) -> Result<()> {
        let url = format!("{}/reserve-segment", self.base_url);
        self.client
            .post(&url)
            .json(&req)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn reserve_slot(&self, slot: Slot) -> Result<SlotId> {
        let url = format!("{}/slots", self.base_url);
        let response = self.client.post(&url).json(&slot).send().await?;
        response.json().await.map_err(Into::into)
    }
}
