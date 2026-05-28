use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SlotInfo {
    pub slot: u64,
    pub parent: u64,
    pub root: u64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LeaderInfo {
    pub slot: u64,
    pub leader: String,
    pub is_jito_leader: bool,
}

pub struct SlotStreamer {
    pub rpc_url: String,
    pub http: Client,
}

impl SlotStreamer {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            http: Client::new(),
        }
    }

    pub async fn get_current_slot(&self) -> Result<u64> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlot",
            "params": [{"commitment": "processed"}]
        });

        let resp = self.http
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let slot = data["result"].as_u64().unwrap_or(0);
        info!("Current slot: {}", slot);
        Ok(slot)
    }

    pub async fn get_slot_leaders(&self, slot: u64, limit: u64) -> Result<Vec<String>> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlotLeaders",
            "params": [slot, limit]
        });

        let resp = self.http
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let leaders = data["result"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        Ok(leaders)
    }

    pub async fn get_blockhash(&self, commitment: &str) -> Result<String> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{"commitment": commitment}]
        });

        let resp = self.http
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let blockhash = data["result"]["value"]["blockhash"]
            .as_str()
            .unwrap_or("")
            .to_string();

        info!("Blockhash ({}): {}", commitment, blockhash);
        Ok(blockhash)
    }

    pub async fn poll_slots(&self, count: u32) -> Result<Vec<SlotInfo>> {
        let mut slots = Vec::new();
        for i in 0..count {
            let slot = self.get_current_slot().await?;
            slots.push(SlotInfo {
                slot,
                parent: slot.saturating_sub(1),
                root: slot.saturating_sub(32),
                timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
            });
            if i < count - 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(400)).await;
            }
        }
        Ok(slots)
    }
}
