//! Domain configuration

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

/// Domain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainConfig {
    /// Shard ID
    pub shard: u64,
    /// Path to keypair file (JSON format)
    #[serde(default = "default_keypair_path")]
    pub keypair_path: String,
    /// RPC URL for delta testnet
    #[serde(default = "default_rpc_url")]
    pub rpc_url: String,
    /// HTTP port for API
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    /// LLM provider ("claude" or "openai")
    #[serde(default = "default_llm_provider")]
    pub llm_provider: String,
    /// LLM API key (optional in config, can be set via env)
    #[serde(default)]
    pub llm_api_key: String,
    /// Whether to use mock mode (no real Delta connection)
    #[serde(default)]
    pub mock_mode: bool,
}

fn default_keypair_path() -> String {
    "keypair.json".to_string()
}

fn default_rpc_url() -> String {
    "http://164.92.69.96:9000".to_string()
}

fn default_api_port() -> u16 {
    8080
}

fn default_llm_provider() -> String {
    "claude".to_string()
}

impl DomainConfig {
    /// Load configuration from a YAML file
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let mut config: Self =
            serde_yaml::from_str(&contents).context("Failed to parse config YAML")?;

        // Override with environment variables if set
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(shard) = env::var("SHARD") {
            if let Ok(s) = shard.parse() {
                self.shard = s;
            }
        }
        if let Ok(keypair) = env::var("KEYPAIR_PATH") {
            self.keypair_path = keypair;
        }
        if let Ok(rpc) = env::var("RPC_URL") {
            self.rpc_url = rpc;
        }
        if let Ok(port) = env::var("API_PORT") {
            if let Ok(p) = port.parse() {
                self.api_port = p;
            }
        }
        if let Ok(provider) = env::var("LLM_PROVIDER") {
            self.llm_provider = provider;
        }
        if let Ok(mock) = env::var("MOCK_MODE") {
            self.mock_mode = mock == "1" || mock.to_lowercase() == "true";
        }

        // Always try to get API key from environment
        if self.llm_api_key.is_empty() {
            self.llm_api_key = match self.llm_provider.as_str() {
                "openai" | "gpt" => env::var("OPENAI_API_KEY").unwrap_or_default(),
                _ => env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            };
        }
    }
}

impl Default for DomainConfig {
    fn default() -> Self {
        let mut config = Self {
            shard: 9,
            keypair_path: default_keypair_path(),
            rpc_url: default_rpc_url(),
            api_port: default_api_port(),
            llm_provider: default_llm_provider(),
            llm_api_key: String::new(),
            mock_mode: true, // Default to mock mode for safety
        };
        config.apply_env_overrides();
        config
    }
}
