mod config;
mod streaming;
mod bundle;
mod lifecycle;
mod agent;

use anyhow::Result;
use dotenv::dotenv;
use tracing::info;
use tracing_subscriber;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
};
use std::fs;

use config::Config;
use streaming::SlotStreamer;
use bundle::{JitoClient, build_versioned_transaction, serialize_versioned};
use lifecycle::{LifecycleEntry, BundleStatus};
use agent::AgentOrchestrator;

// Jito tip accounts (mainnet)
const JITO_TIP_ACCOUNTS: &[&str] = &[
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt13gdCTBL",
];

fn load_keypair(path: &str) -> Result<Keypair> {
    let data = fs::read_to_string(path)?;
    let bytes: Vec<u8> = serde_json::from_str(&data)?;
    let keypair = Keypair::from_bytes(&bytes)?;
    Ok(keypair)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("[BundleIQ] Starting...");

    let config = Config::from_env()?;
    info!("[Config] RPC: {}", config.solana_rpc);

    // Load keypair
    let keypair = load_keypair(&config.wallet_keypair_path)?;
    info!("[Wallet] Public key: {}", keypair.pubkey());

    let streamer = SlotStreamer::new(config.solana_rpc.clone());
    let jito = JitoClient::new(config.jito_block_engine.clone());
    let agent = AgentOrchestrator::new(config.openai_api_key.clone());

    // Step 1: Stream slots
    info!("[Stream] Polling slots...");
    let slots = streamer.poll_slots(3).await?;
    let current_slot = slots.last().map(|s| s.slot).unwrap_or(0);
    info!("[Stream] Current slot: {}", current_slot);

    // Step 2: Get leaders
    info!("[Leaders] Fetching schedule...");
    let leaders = streamer.get_slot_leaders(current_slot, 10).await?;
    info!("[Leaders] Upcoming: {:?}", &leaders[..leaders.len().min(5)]);

    let jito_leaders = vec![
        "j1t0S8Y2rKRBMmMV6RRPF2QdFoJzJ5Qh1hG3nK9wXe".to_string(),
    ];

    // Step 3: Get real tip floor
    let tip_floor = jito.get_tip_floor().await.unwrap_or(1000);

    // Step 4: Agent decides tip
    info!("[Agent:Tip] Deciding tip...");
    let tip_lamports = agent.get_tip(current_slot, tip_floor, "low").await?;
    info!("[Agent:Tip] Decided: {} lamports", tip_lamports);

    // Step 5: Agent decides timing
    info!("[Agent:Timing] Deciding submission window...");
    let wait_slots = agent.get_timing(
        current_slot,
        &leaders,
        &jito_leaders,
        150,
    ).await?;

    if wait_slots > 0 {
        info!("[Agent:Timing] Waiting {} slots...", wait_slots);
        tokio::time::sleep(
            tokio::time::Duration::from_millis(wait_slots * 400)
        ).await;
    }

    // Step 6: Fetch blockhash (confirmed — not finalized)
    info!("[Blockhash] Fetching confirmed blockhash...");
    let blockhash_str = streamer.get_blockhash("confirmed").await?;
    let blockhash: solana_sdk::hash::Hash = blockhash_str.parse()?;
    info!("[Blockhash] {}", blockhash_str);

    // Step 7: Build real versioned transactions
    info!("[Tx] Building transactions...");

    // Main tx: send 1000 lamports to self (devnet test)
    let main_tx = build_versioned_transaction(
        &keypair,
        &keypair.pubkey().to_string(),
        500,
        blockhash,
    )?;

    // Tip tx: tip a Jito account
    let tip_account = JITO_TIP_ACCOUNTS[0];
    let tip_tx = build_versioned_transaction(
        &keypair,
        tip_account,
        tip_lamports,
        blockhash,
    )?;

    let serialized = vec![
        serialize_versioned(&main_tx)?,
        serialize_versioned(&tip_tx)?,
    ];

    info!("[Tx] Built {} transactions", serialized.len());

    // Step 8: Submit with retry loop
    let mut attempt = 0u32;
    let max_attempts = 3;
    let mut current_tip = tip_lamports;
    let bundle_id = format!("bundle_{}", current_slot);

    let mut entry = LifecycleEntry::new(
        bundle_id.clone(),
        current_slot,
        current_tip,
    );

    loop {
        attempt += 1;
        info!("[Bundle] Attempt {} | Tip: {} lamports", attempt, current_tip);

        let result = jito.submit_bundle(serialized.clone(), current_tip).await;

        match result {
            Ok(bundle_result) => {
                info!("[Bundle] Submitted: {}", bundle_result.bundle_id);
                entry.update_status(BundleStatus::Processed);
                entry.save_to_log()?;

                for _ in 0..10 {
                    tokio::time::sleep(
                        tokio::time::Duration::from_secs(2)
                    ).await;
                    let status = jito.get_bundle_status(&bundle_result.bundle_id).await?;
                    info!("[Bundle] Status: {}", status);

                    match status.as_str() {
                        "confirmed" => {
                            entry.update_status(BundleStatus::Confirmed);
                            entry.save_to_log()?;
                        }
                        "finalized" => {
                            entry.update_status(BundleStatus::Finalized);
                            entry.save_to_log()?;
                            info!("[Bundle] Finalized!");
                            break;
                        }
                        _ => {}
                    }
                }
                break;
            }
            Err(e) => {
                let error_msg = e.to_string();
                info!("[Bundle] Failed: {}", error_msg);
                entry.update_status(BundleStatus::Failed(error_msg.clone()));
                entry.agent_reasoning = Some(error_msg.clone());
                entry.save_to_log()?;

                if attempt >= max_attempts {
                    info!("[Bundle] Max attempts reached.");
                    break;
                }

                let (should_retry, refresh_bh, new_tip) = agent
                    .handle_failure(&error_msg, current_slot, current_tip, attempt)
                    .await?;

                if !should_retry {
                    info!("[Agent:Failure] Decided not to retry.");
                    break;
                }

                current_tip = new_tip;

                if refresh_bh {
                    info!("[Blockhash] Refreshing...");
                    let _new_bh = streamer.get_blockhash("confirmed").await?;
                }

                tokio::time::sleep(
                    tokio::time::Duration::from_secs(1)
                ).await;
            }
        }
    }

    info!("[BundleIQ] Run complete. Logs at logs/lifecycle.jsonl");
    Ok(())
}
