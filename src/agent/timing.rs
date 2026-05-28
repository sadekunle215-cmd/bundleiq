use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tracing::info;

pub struct TimingAgent {
    pub client: Client,
    pub api_key: String,
}

#[derive(Debug)]
pub struct TimingDecision {
    pub submit_now: bool,
    pub wait_slots: u64,
    pub reasoning: String,
}

impl TimingAgent {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn decide_timing(
        &self,
        current_slot: u64,
        upcoming_leaders: &[String],
        known_jito_leaders: &[String],
        slots_until_expiry: u64,
    ) -> Result<TimingDecision> {
        let jito_slots: Vec<usize> = upcoming_leaders
            .iter()
            .enumerate()
            .filter(|(_, l)| known_jito_leaders.contains(l))
            .map(|(i, _)| i)
            .collect();

        let prompt = format!(
            r#"You are a Solana bundle submission timing agent.

Current state:
- Current slot: {}
- Upcoming leaders (next 10 slots): {:?}
- Known Jito leaders: {:?}
- Jito leader positions in schedule: {:?}
- Slots until blockhash expiry: {}

Decide whether to submit the bundle now or wait for a better slot.

Respond in this exact JSON format:
{{
  "submit_now": true/false,
  "wait_slots": <number 0-10>,
  "reasoning": "detailed explanation of timing decision"
}}

Consider:
- Submit now if a Jito leader is within next 2 slots
- Wait if non-Jito leader is next but Jito leader is coming in 2-4 slots
- Always submit if blockhash expires in less than 10 slots
- Never wait more than 5 slots"#,
            current_slot,
            upcoming_leaders,
            known_jito_leaders,
            jito_slots,
            slots_until_expiry
        );

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a Solana bundle timing expert. Always respond with valid JSON only."
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "temperature": 0.2,
                "max_tokens": 400
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}");

        info!("Timing agent reasoning: {}", content);

        let parsed: serde_json::Value = serde_json::from_str(content)
            .unwrap_or(json!({}));

        Ok(TimingDecision {
            submit_now: parsed["submit_now"].as_bool().unwrap_or(true),
            wait_slots: parsed["wait_slots"].as_u64().unwrap_or(0),
            reasoning: parsed["reasoning"]
                .as_str()
                .unwrap_or("No reasoning provided")
                .to_string(),
        })
    }
}
