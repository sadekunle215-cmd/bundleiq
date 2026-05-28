use anyhow::Result;
use reqwest::Client;
use serde_json::json;
use tracing::info;

pub struct FailureAgent {
    pub client: Client,
    pub api_key: String,
}

#[derive(Debug)]
pub struct RetryDecision {
    pub should_retry: bool,
    pub refresh_blockhash: bool,
    pub increase_tip: bool,
    pub new_tip_multiplier: f64,
    pub reasoning: String,
}

impl FailureAgent {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn analyze_failure(
        &self,
        error: &str,
        slot: u64,
        tip_lamports: u64,
        attempt: u32,
    ) -> Result<RetryDecision> {
        let prompt = format!(
            r#"You are an expert Solana transaction infrastructure agent.

A Jito bundle submission failed with this error:
Error: {}
Slot: {}
Tip: {} lamports
Attempt: {}

Analyze the failure and decide the retry strategy.
Respond in this exact JSON format:
{{
  "should_retry": true/false,
  "refresh_blockhash": true/false,
  "increase_tip": true/false,
  "new_tip_multiplier": 1.0-3.0,
  "reasoning": "detailed explanation of why this failed and what to change"
}}

Consider:
- BlockhashNotFound or expired blockhash → refresh blockhash
- Bundle dropped or low tip → increase tip
- Leader skipped slot → retry with same params
- Max retries exceeded → do not retry
- Compute exceeded → do not retry, need code fix"#,
            error, slot, tip_lamports, attempt
        );

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": "gpt-4",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a Solana transaction infrastructure expert. Always respond with valid JSON only."
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "temperature": 0.2,
                "max_tokens": 500
            }))
            .send()
            .await?;

        let data: serde_json::Value = response.json().await?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}");

        info!("Failure agent reasoning: {}", content);

        let parsed: serde_json::Value = serde_json::from_str(content)
            .unwrap_or(json!({}));

        Ok(RetryDecision {
            should_retry: parsed["should_retry"].as_bool().unwrap_or(false),
            refresh_blockhash: parsed["refresh_blockhash"].as_bool().unwrap_or(true),
            increase_tip: parsed["increase_tip"].as_bool().unwrap_or(false),
            new_tip_multiplier: parsed["new_tip_multiplier"].as_f64().unwrap_or(1.0),
            reasoning: parsed["reasoning"]
                .as_str()
                .unwrap_or("No reasoning provided")
                .to_string(),
        })
    }
}
