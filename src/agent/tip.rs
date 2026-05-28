use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tracing::info;

pub struct TipAgent {
    pub client: Client,
    pub api_key: String,
}

#[derive(Debug)]
pub struct TipDecision {
    pub tip_lamports: u64,
    pub confidence: f64,
    pub reasoning: String,
}

impl TipAgent {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn decide_tip(
        &self,
        current_slot: u64,
        recent_tip_floor: u64,
        network_congestion: &str,
        priority: &str,
    ) -> Result<TipDecision> {
        let prompt = format!(
            r#"You are a Solana Jito tip optimization agent.

Current network conditions:
- Slot: {}
- Recent tip floor: {} lamports
- Network congestion: {}
- Transaction priority: {}

Decide the optimal tip amount in lamports to maximize landing probability while minimizing cost.

Respond in this exact JSON format:
{{
  "tip_lamports": <number>,
  "confidence": <0.0-1.0>,
  "reasoning": "detailed explanation balancing cost vs landing probability"
}}

Guidelines:
- Minimum tip: 1000 lamports
- Low congestion + low priority: 1000-5000 lamports
- Medium congestion + medium priority: 5000-50000 lamports  
- High congestion + high priority: 50000-200000 lamports
- Always stay above tip floor
- Factor in that higher tips = better bundle placement"#,
            current_slot, recent_tip_floor, network_congestion, priority
        );

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a Solana tip optimization expert. Always respond with valid JSON only."
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

        info!("Tip agent reasoning: {}", content);

        let parsed: serde_json::Value = serde_json::from_str(content)
            .unwrap_or(json!({}));

        Ok(TipDecision {
            tip_lamports: parsed["tip_lamports"].as_u64().unwrap_or(5000),
            confidence: parsed["confidence"].as_f64().unwrap_or(0.5),
            reasoning: parsed["reasoning"]
                .as_str()
                .unwrap_or("No reasoning provided")
                .to_string(),
        })
    }
}
