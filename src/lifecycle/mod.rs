use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BundleStatus {
    Submitted,
    Processed,
    Confirmed,
    Finalized,
    Failed(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LifecycleEntry {
    pub bundle_id: String,
    pub slot: u64,
    pub status: BundleStatus,
    pub timestamp: DateTime<Utc>,
    pub tip_lamports: u64,
    pub commitment_progression: Vec<String>,
    pub failure_reason: Option<String>,
    pub agent_reasoning: Option<String>,
}

impl LifecycleEntry {
    pub fn new(bundle_id: String, slot: u64, tip_lamports: u64) -> Self {
        Self {
            bundle_id,
            slot,
            status: BundleStatus::Submitted,
            timestamp: Utc::now(),
            tip_lamports,
            commitment_progression: vec!["submitted".to_string()],
            failure_reason: None,
            agent_reasoning: None,
        }
    }

    pub fn update_status(&mut self, status: BundleStatus) {
        let label = match &status {
            BundleStatus::Processed => "processed".to_string(),
            BundleStatus::Confirmed => "confirmed".to_string(),
            BundleStatus::Finalized => "finalized".to_string(),
            BundleStatus::Failed(r) => format!("failed: {}", r),
            BundleStatus::Submitted => "submitted".to_string(),
        };
        self.commitment_progression.push(label);
        self.status = status;
    }

    pub fn save_to_log(&self) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/lifecycle.jsonl")?;
        let line = serde_json::to_string(self)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }
}
