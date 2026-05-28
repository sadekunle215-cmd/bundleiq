use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::{
    hash::Hash,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::Transaction,
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

    pub async fn get_tip_accounts(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/v1/bundles/tip_floor", self.block_engine_url);
        let resp = self.http.get(&url).send().await?;
        let data: serde_json::Value = resp.json().await?;
        let accounts = vec![
            "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5".to_string(),
            "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".to_string(),
            "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY".to_string(),
            "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt13gdCTBL".to_string(),
        ];
        info!("Tip floor data: {:?}", data);
        Ok(accounts)
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
        let bundle_id = data["result"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        info!("Bundle submitted: {}", bundle_id);

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

pub fn build_tip_transaction(
    payer: &Keypair,
    tip_account: &str,
    tip_lamports: u64,
    recent_blockhash: Hash,
) -> Result<Transaction> {
    let tip_pubkey = tip_account.parse()?;
    let ix = system_instruction::transfer(
        &payer.pubkey(),
        &tip_pubkey,
        tip_lamports,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );
    Ok(tx)
}
