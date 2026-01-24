//! Domain configuration

use serde::{Deserialize, Serialize};
use std::env;

/// Domain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    /// Shard ID
    pub shard: u64,
    /// Path to keypair file
    pub keypair_path: String,
    /// RPC URL for delta testnet
    pub rpc_url: String,
    /// HTTP port for API
    pub api_port: u16,
    /// LLM provider ("claude" or "gpt")
    pub llm_provider: String,
    /// LLM API key
    pub llm_api_key: String,
    /// Use mock compiler (for demo without LLM)
    pub use_mock_compiler: bool,
}

impl Default for DomainConfig {
    fn default() -> Self {
        Self {
            shard: env::var("SHARD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(9),
            keypair_path: env::var("KEYPAIR_PATH")
                .unwrap_or_else(|_| "keypair_9.json".to_string()),
            rpc_url: env::var("RPC_URL")
                .unwrap_or_else(|_| "http://164.92.69.96:9000".to_string()),
            api_port: env::var("API_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3000),
            llm_provider: env::var("LLM_PROVIDER")
                .unwrap_or_else(|_| "claude".to_string()),
            llm_api_key: env::var("ANTHROPIC_API_KEY")
                .or_else(|_| env::var("OPENAI_API_KEY"))
                .unwrap_or_default(),
            use_mock_compiler: env::var("USE_MOCK_COMPILER")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
        }
    }
}
