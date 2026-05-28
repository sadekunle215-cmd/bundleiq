pub mod failure;
pub mod tip;
pub mod timing;

use anyhow::Result;
use tracing::info;

use crate::agent::failure::FailureAgent;
use crate::agent::tip::TipAgent;
use crate::agent::timing::TimingAgent;

pub struct AgentOrchestrator {
    pub failure_agent: FailureAgent,
    pub tip_agent: TipAgent,
    pub timing_agent: TimingAgent,
}

impl AgentOrchestrator {
    pub fn new(api_key: String) -> Self {
        Self {
            failure_agent: FailureAgent::new(api_key.clone()),
            tip_agent: TipAgent::new(api_key.clone()),
            timing_agent: TimingAgent::new(api_key),
        }
    }

    pub async fn get_tip(&self, slot: u64, tip_floor: u64, congestion: &str) -> Result<u64> {
        let decision = self.tip_agent
            .decide_tip(slot, tip_floor, congestion, "medium")
            .await?;
        info!("Tip decision: {} lamports | Reason: {}", decision.tip_lamports, decision.reasoning);
        Ok(decision.tip_lamports)
    }

    pub async fn get_timing(
        &self,
        slot: u64,
        leaders: &[String],
        jito_leaders: &[String],
        expiry: u64,
    ) -> Result<u64> {
        let decision = self.timing_agent
            .decide_timing(slot, leaders, jito_leaders, expiry)
            .await?;
        info!("Timing decision: submit_now={} wait={} | Reason: {}", 
            decision.submit_now, decision.wait_slots, decision.reasoning);
        if decision.submit_now {
            Ok(0)
        } else {
            Ok(decision.wait_slots)
        }
    }

    pub async fn handle_failure(
        &self,
        error: &str,
        slot: u64,
        tip: u64,
        attempt: u32,
    ) -> Result<(bool, bool, u64)> {
        let decision = self.failure_agent
            .analyze_failure(error, slot, tip, attempt)
            .await?;
        info!("Failure decision: retry={} refresh={} | Reason: {}", 
            decision.should_retry, decision.refresh_blockhash, decision.reasoning);
        let new_tip = if decision.increase_tip {
            (tip as f64 * decision.new_tip_multiplier) as u64
        } else {
            tip
        };
        Ok((decision.should_retry, decision.refresh_blockhash, new_tip))
    }
}
