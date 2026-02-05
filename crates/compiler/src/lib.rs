//! LLM-based ESC Compiler
//!
//! Compiles English quote text into structured QuoteConstraints.
//! Uses Claude or GPT to parse natural language into guardrails.

use rfq_models::{QuoteConstraints, QuoteSpec, Side};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("Failed to parse quote text: {0}")]
    ParseError(String),
    #[error("LLM API error: {0}")]
    ApiError(String),
    #[error("Invalid constraint: {0}")]
    InvalidConstraint(String),
}

/// Response from the LLM containing parsed quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedQuote {
    pub asset: String,
    pub size: f64,
    pub side: String,
    pub max_price: Option<f64>,
    pub min_price: Option<f64>,
    pub currency: String,
    pub expiry_minutes: u64,
    pub allowed_sources: Vec<String>,
    pub max_staleness_seconds: u64,
    pub quorum_required: u32,
    pub quorum_tolerance_percent: f64,
    pub allowed_takers: Vec<String>,
    pub no_side_payments: bool,
    pub atomic_dvp_only: bool,
}

/// Configuration for the LLM compiler
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// Which LLM to use ("claude" or "gpt")
    pub llm: String,
    /// API key
    pub api_key: String,
    /// Model name
    pub model: String,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            llm: "claude".to_string(),
            api_key: String::new(),
            model: "claude-3-5-sonnet-20241022".to_string(),
        }
    }
}

/// The ESC Compiler
pub struct Compiler {
    config: CompilerConfig,
    client: reqwest::Client,
}

impl Compiler {
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    /// Compile English text into QuoteSpec and QuoteConstraints
    pub async fn compile(
        &self,
        text: &str,
        quote_id: [u8; 32],
        nonce: u64,
    ) -> Result<(QuoteSpec, QuoteConstraints), CompilerError> {
        let parsed = self.call_llm(text).await?;
        self.build_constraints(parsed, quote_id, nonce)
    }

    /// Build the LLM prompt
    fn build_prompt(&self, text: &str) -> String {
        format!(
            r#"You are a quote compiler for an OTC trading system. Extract structured terms from the following quote.

Quote: "{}"

Return JSON with:
- asset: string (e.g., "dETH")
- size: number
- side: "buy" or "sell"
- max_price: number or null
- min_price: number or null
- currency: string (e.g., "USDD")
- expiry_minutes: number
- allowed_sources: string[] (price feed names like "FeedA", "FeedB")
- max_staleness_seconds: number (default 60)
- quorum_required: number (default 1)
- quorum_tolerance_percent: number (default 1.0)
- allowed_takers: string[] (empty means any)
- no_side_payments: boolean
- atomic_dvp_only: boolean

Return ONLY valid JSON, no markdown code blocks."#,
            text
        )
    }

    /// Call the LLM API
    async fn call_llm(&self, text: &str) -> Result<ParsedQuote, CompilerError> {
        let prompt = self.build_prompt(text);

        let response = match self.config.llm.as_str() {
            "claude" => self.call_claude(&prompt).await?,
            "gpt" => self.call_gpt(&prompt).await?,
            _ => return Err(CompilerError::ApiError("Unknown LLM".to_string())),
        };

        serde_json::from_str(&response)
            .map_err(|e| CompilerError::ParseError(format!("JSON parse error: {}", e)))
    }

    async fn call_claude(&self, prompt: &str) -> Result<String, CompilerError> {
        #[derive(Serialize)]
        struct ClaudeRequest {
            model: String,
            max_tokens: u32,
            messages: Vec<ClaudeMessage>,
        }

        #[derive(Serialize)]
        struct ClaudeMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct ClaudeResponse {
            content: Vec<ClaudeContent>,
        }

        #[derive(Deserialize)]
        struct ClaudeContent {
            text: String,
        }

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            max_tokens: 1024,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| CompilerError::ApiError(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| CompilerError::ApiError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(CompilerError::ApiError(format!(
                "Claude API error ({}): {}",
                status, body
            )));
        }

        let claude_response: ClaudeResponse = serde_json::from_str(&body)
            .map_err(|e| CompilerError::ApiError(format!("Failed to parse response: {} - body: {}", e, body)))?;

        claude_response
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| CompilerError::ApiError("Empty response".to_string()))
    }

    async fn call_gpt(&self, prompt: &str) -> Result<String, CompilerError> {
        #[derive(Serialize)]
        struct GptRequest {
            model: String,
            messages: Vec<GptMessage>,
        }

        #[derive(Serialize, Deserialize)]
        struct GptMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct GptResponse {
            choices: Vec<GptChoice>,
        }

        #[derive(Deserialize)]
        struct GptChoice {
            message: GptMessage,
        }

        let request = GptRequest {
            model: self.config.model.clone(),
            messages: vec![GptMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| CompilerError::ApiError(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| CompilerError::ApiError(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(CompilerError::ApiError(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        let gpt_response: GptResponse = serde_json::from_str(&body)
            .map_err(|e| CompilerError::ApiError(format!("Failed to parse response: {} - body: {}", e, body)))?;

        gpt_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| CompilerError::ApiError("Empty response".to_string()))
    }

    /// Build QuoteSpec and QuoteConstraints from parsed quote
    fn build_constraints(
        &self,
        parsed: ParsedQuote,
        quote_id: [u8; 32],
        nonce: u64,
    ) -> Result<(QuoteSpec, QuoteConstraints), CompilerError> {
        let side = match parsed.side.to_lowercase().as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => return Err(CompilerError::InvalidConstraint("Invalid side".to_string())),
        };

        let spec = QuoteSpec {
            asset: parsed.asset.clone(),
            size: parsed.size,
            side,
            limit_price: parsed.max_price.or(parsed.min_price),
            currency: parsed.currency.clone(),
        };

        // Convert to smallest units (assuming 9 decimal places)
        let size_units = (parsed.size * 1_000_000_000.0) as u64;
        let max_debit = match (parsed.max_price, side) {
            (Some(price), Side::Buy) => (price * parsed.size * 1_000_000_000.0) as u64,
            _ => u64::MAX,
        };

        let now = chrono::Utc::now().timestamp() as u64;
        let expiry = now + (parsed.expiry_minutes * 60);

        let constraints = QuoteConstraints {
            quote_id,
            max_debit,
            min_credit: parsed.min_price.map(|p| (p * parsed.size * 1_000_000_000.0) as u64),
            expiry_timestamp: expiry,
            allowed_sources: parsed.allowed_sources,
            max_staleness_secs: parsed.max_staleness_seconds,
            quorum_count: parsed.quorum_required,
            quorum_tolerance_percent: parsed.quorum_tolerance_percent,
            allowed_takers: parsed.allowed_takers,
            allowed_assets: vec![parsed.asset],
            require_atomic_dvp: parsed.atomic_dvp_only,
            no_side_payments: parsed.no_side_payments,
            nonce,
            max_fill_size: size_units,
        };

        Ok((spec, constraints))
    }
}

/// Generate a human-readable summary of constraints
pub fn summarize_constraints(constraints: &QuoteConstraints) -> String {
    let mut parts = vec![];

    parts.push(format!(
        "Max debit: {} units",
        constraints.max_debit
    ));

    if let Some(min) = constraints.min_credit {
        parts.push(format!("Min credit: {} units", min));
    }

    parts.push(format!(
        "Expires: {}",
        constraints.expiry_datetime()
    ));

    if !constraints.allowed_sources.is_empty() {
        parts.push(format!(
            "Allowed feeds: {}",
            constraints.allowed_sources.join(", ")
        ));
    }

    parts.push(format!(
        "Feed freshness: <{}s",
        constraints.max_staleness_secs
    ));

    if constraints.quorum_count > 1 {
        parts.push(format!(
            "Quorum: {} sources within {}%",
            constraints.quorum_count, constraints.quorum_tolerance_percent
        ));
    }

    if !constraints.allowed_takers.is_empty() {
        parts.push(format!(
            "Allowed takers: {}",
            constraints.allowed_takers.join(", ")
        ));
    }

    if constraints.require_atomic_dvp {
        parts.push("Requires atomic DvP".to_string());
    }

    if constraints.no_side_payments {
        parts.push("No side-payments allowed".to_string());
    }

    parts.join(" | ")
}
