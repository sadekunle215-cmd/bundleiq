use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::{
    hash::Hash,
    message::{v0, VersionedMessage},
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::{Transaction, VersionedTransaction},
};

use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleResult {
    pub bundle_id: String,
    pub slot: u64,
    pub tip_lamports: u64,
}

pub struct JitoClient {
    pub http: Client,
    pub block_engine_url: String,
}

impl JitoClient {
    pub fn new(block_engine_url: String) -> Self {
        Self {
            http: Client::new(),
            block_engine_url,
        }
    }

    pub async fn get_tip_floor(&self) -> Result<u64> {
        let url = format!("{}/api/v1/bundles/tip_floor", self.block_engine_url);
        let resp = self.http.get(&url).send().await?;
        let data: serde_json::Value = resp.json().await?;
        let floor = data[0]["landed_tips_50th_percentile"]
            .as_f64()
            .unwrap_or(1000.0) as u64;
        info!("[Jito] Tip floor: {} lamports", floor);
        Ok(floor)
    }

    pub async fn submit_bundle(
        &self,
        transactions: Vec<String>,
        tip_lamports: u64,
    ) -> Result<BundleResult> {
        let url = format!("{}/api/v1/bundles", self.block_engine_url);

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendBundle",
            "params": [transactions]
        });

        let resp = self.http
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;

        if let Some(err) = data.get("error") {
            return Err(anyhow::anyhow!("Jito error: {}", err));
        }

        let bundle_id = data["result"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        info!("[Jito] Bundle submitted: {}", bundle_id);

        Ok(BundleResult {
            bundle_id,
            slot: 0,
            tip_lamports,
        })
    }

    pub async fn get_bundle_status(&self, bundle_id: &str) -> Result<String> {
        let url = format!("{}/api/v1/bundles", self.block_engine_url);
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBundleStatuses",
            "params": [[bundle_id]]
        });

        let resp = self.http
            .post(&url)
            .json(&payload)
            .send()
            .await?;

        let data: serde_json::Value = resp.json().await?;
        let status = data["result"]["value"][0]["confirmation_status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(status)
    }
}

pub fn build_versioned_transaction(
    payer: &Keypair,
    to: &str,
    lamports: u64,
    recent_blockhash: Hash,
) -> Result<VersionedTransaction> {
    let to_pubkey = to.parse()?;
    let ix = system_instruction::transfer(
        &payer.pubkey(),
        &to_pubkey,
        lamports,
    );
    let message = v0::Message::try_compile(
        &payer.pubkey(),
        &[ix],
        &[],
        recent_blockhash,
    )?;
    let tx = VersionedTransaction::try_new(
        VersionedMessage::V0(message),
        &[payer],
    )?;
    Ok(tx)
}

pub fn serialize_versioned(tx: &VersionedTransaction) -> Result<String> {
    let bytes = bincode::serialize(tx)?;
    Ok(bs58::encode(bytes).into_string())
}
