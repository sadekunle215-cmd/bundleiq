mod config;
mod streaming;
mod bundle;
mod lifecycle;
mod agent;

use anyhow::Result;
use dotenv::dotenv;
use tracing::info;
use tracing_subscriber;

use config::Config;
use streaming::SlotStreamer;
use bundle::JitoClient;
use lifecycle::{LifecycleEntry, BundleStatus};
use agent::AgentOrchestrator;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    info!("[BundleIQ] Starting...");

    let config = Config::from_env()?;
    info!("[Config] RPC: {}", config.solana_rpc);

    let streamer = SlotStreamer::new(config.solana_rpc.clone());
    let jito = JitoClient::new(config.jito_block_engine.clone());
    let agent = AgentOrchestrator::new(config.openai_api_key.clone());

    info!("[Stream] Polling slots...");
    let slots = streamer.poll_slots(3).await?;
    let current_slot = slots.last().map(|s| s.slot).unwrap_or(0);
    info!("[Stream] Current slot: {}", current_slot);

    info!("[Leaders] Fetching schedule...");
    let leaders = streamer.get_slot_leaders(current_slot, 10).await?;
    info!("[Leaders] Upcoming: {:?}", &leaders[..leaders.len().min(5)]);

    let jito_leaders = vec![
        "j1t0S8Y2rKRBMmMV6RRPF2QdFoJzJ5Qh1hG3nK9wXe".to_string(),
        "J1to5PufSuMgCDGBqEgTNAVkYGymDjHMcGGsMBr67mT".to_string(),
    ];

    info!("[Agent:Tip] Deciding tip...");
    let tip_lamports = agent.get_tip(current_slot, 1000, "medium").await?;
    info!("[Agent:Tip] Decided: {} lamports", tip_lamports);

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

    info!("[Blockhash] Fetching confirmed blockhash...");
    let blockhash = streamer.get_blockhash("confirmed").await?;
    info!("[Blockhash] {}", blockhash);

    info!("[Bundle] Submitting...");
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

        let result = jito.submit_bundle(
            vec!["simulated_tx_base64".to_string()],
            current_tip,
        ).await;

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
                            info!("[Bundle] Finalized.");
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
