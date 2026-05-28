use anyhow::Result;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub solana_rpc: String,
    pub openai_api_key: String,
    pub jito_block_engine: String,
    pub wallet_keypair_path: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            solana_rpc: env::var("SOLANA_RPC")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            openai_api_key: env::var("OPENAI_API_KEY")
                .expect("OPENAI_API_KEY must be set"),
            jito_block_engine: env::var("JITO_BLOCK_ENGINE")
                .unwrap_or_else(|_| "https://mainnet.block-engine.jito.wtf".to_string()),
            wallet_keypair_path: env::var("WALLET_KEYPAIR_PATH")
                .unwrap_or_else(|_| "./keypair.json".to_string()),
        })
    }
}
