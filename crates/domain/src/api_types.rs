//! Flattened API response types for bot consumption
//!
//! These types provide a simpler, flatter structure that's easier for
//! LLM agents to understand and work with. They transform the internal
//! rich models into concise API responses.

use rfq_models::{FillReceipt, FillResult, Quote, QuoteConstraints, QuoteStatus, Side};
use serde::{Deserialize, Serialize};

// ============================================================================
// Quote Types (Flattened)
// ============================================================================

/// Flattened quote for API responses - easier for bots to parse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiQuote {
    /// Unique quote ID
    pub id: String,
    /// Original English text
    pub text: String,
    /// Current status: "active", "filled", "expired", "cancelled"
    pub status: String,
    /// Asset being traded (e.g., "dETH")
    pub asset: String,
    /// Trade direction: "buy" or "sell"
    pub direction: String,
    /// Size of the trade
    pub size: f64,
    /// Price limit (max for buys, min for sells)
    pub price_limit: Option<f64>,
    /// Settlement currency (e.g., "USDD")
    pub currency: String,
    /// Expiry as unix timestamp (seconds)
    pub expires_at: i64,
    /// Creation time as unix timestamp (seconds)
    pub created_at: i64,
    /// Maker's owner ID
    pub maker_owner_id: String,
    /// Maker's shard number
    pub maker_shard: u64,
    /// The compiled constraints (Local Law)
    pub local_law: ApiLocalLaw,
}

/// Flattened Local Law (constraints) for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiLocalLaw {
    /// Maximum amount that can be debited (in plancks)
    pub max_debit: u64,
    /// Expiry timestamp
    pub expiry_timestamp: u64,
    /// Allowed price feed sources
    pub allowed_sources: Vec<String>,
    /// Maximum staleness for price feeds (seconds)
    pub max_staleness_secs: u64,
    /// Minimum number of sources required
    pub quorum_count: u32,
    /// Maximum price spread tolerance (percentage)
    pub quorum_tolerance_percent: f64,
    /// Require atomic delivery vs payment
    pub require_atomic_dvp: bool,
    /// Disallow extra transfers
    pub no_side_payments: bool,
}

impl From<&Quote> for ApiQuote {
    fn from(q: &Quote) -> Self {
        // Extract shard from vault address (format: "owner_id,shard")
        let maker_shard = q
            .maker_vault_address
            .split(',')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Self {
            id: q.id.to_string(),
            text: q.original_text.clone(),
            status: status_to_string(q.status),
            asset: q.spec.asset.clone(),
            direction: side_to_string(q.spec.side),
            size: q.spec.size,
            price_limit: q.spec.limit_price,
            currency: q.spec.currency.clone(),
            expires_at: q.expires_at.timestamp(),
            created_at: q.created_at.timestamp(),
            maker_owner_id: q.maker_owner_id.clone(),
            maker_shard,
            local_law: ApiLocalLaw::from(&q.constraints),
        }
    }
}

impl From<&QuoteConstraints> for ApiLocalLaw {
    fn from(c: &QuoteConstraints) -> Self {
        Self {
            max_debit: c.max_debit,
            expiry_timestamp: c.expiry_timestamp,
            allowed_sources: c.allowed_sources.clone(),
            max_staleness_secs: c.max_staleness_secs,
            quorum_count: c.quorum_count,
            quorum_tolerance_percent: c.quorum_tolerance_percent,
            require_atomic_dvp: c.require_atomic_dvp,
            no_side_payments: c.no_side_payments,
        }
    }
}

fn status_to_string(status: QuoteStatus) -> String {
    match status {
        QuoteStatus::Active => "active".to_string(),
        QuoteStatus::Filled => "filled".to_string(),
        QuoteStatus::Expired => "expired".to_string(),
        QuoteStatus::Cancelled => "cancelled".to_string(),
    }
}

fn side_to_string(side: Side) -> String {
    match side {
        Side::Buy => "buy".to_string(),
        Side::Sell => "sell".to_string(),
    }
}

// ============================================================================
// Create Quote Response
// ============================================================================

/// Response after creating a quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCreateQuoteResponse {
    /// The created quote (flattened)
    #[serde(flatten)]
    pub quote: ApiQuote,
    /// Human-readable summary of constraints
    pub constraints_summary: String,
    /// Success message
    pub message: String,
}

// ============================================================================
// Fill Response Types
// ============================================================================

/// Response after attempting to fill a quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiFillResponse {
    /// Whether the fill succeeded
    pub success: bool,
    /// Fill attempt ID
    pub fill_id: String,
    /// Quote ID that was filled
    pub quote_id: String,
    /// Human-readable message
    pub message: String,
    /// Error details if rejected (null if success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiFillError>,
    /// Receipt details if accepted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<ApiReceipt>,
    /// Proof info if accepted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<ApiProof>,
}

/// Error details for rejected fills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiFillError {
    /// Error code (e.g., "STALE_FEED", "QUORUM_NOT_MET")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Receipt for successful fills
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiReceipt {
    /// Receipt ID
    pub id: String,
    /// Quote ID
    pub quote_id: String,
    /// Taker's owner ID
    pub taker_owner_id: String,
    /// Taker's shard
    pub taker_shard: u64,
    /// Fill size
    pub size: f64,
    /// Fill price
    pub price: f64,
    /// When the fill was executed (unix timestamp)
    pub filled_at: i64,
    /// Settlement details
    pub settlement: Option<ApiSettlement>,
}

/// Settlement details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSettlement {
    /// Amount debited from maker (plancks)
    pub maker_debit: u64,
    /// Amount credited to maker (plancks)
    pub maker_credit: u64,
    /// Amount debited from taker (plancks)
    pub taker_debit: u64,
    /// Amount credited to taker (plancks)
    pub taker_credit: u64,
    /// Asset transferred
    pub asset: String,
    /// Currency transferred
    pub currency: String,
}

/// Proof information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProof {
    /// SDL hash from Delta
    pub sdl_hash: String,
    /// Proof status
    pub status: String,
}

impl From<&FillReceipt> for ApiFillResponse {
    fn from(receipt: &FillReceipt) -> Self {
        let fill_id = receipt.fill_attempt.id.to_string();
        let quote_id = receipt.quote.id.to_string();

        match &receipt.result {
            FillResult::Accepted {
                sdl_hash,
                settlement,
                ..
            } => Self {
                success: true,
                fill_id,
                quote_id,
                message: "Fill accepted! The fill satisfied all Local Law constraints.".to_string(),
                error: None,
                receipt: Some(ApiReceipt {
                    id: receipt.receipt_id.to_string(),
                    quote_id: receipt.quote.id.to_string(),
                    taker_owner_id: receipt.fill_attempt.taker_owner_id.clone(),
                    taker_shard: receipt.fill_attempt.taker_shard,
                    size: receipt.fill_attempt.size,
                    price: receipt.fill_attempt.price,
                    filled_at: receipt.generated_at.timestamp(),
                    settlement: Some(ApiSettlement {
                        maker_debit: settlement.maker_debit,
                        maker_credit: settlement.maker_credit,
                        taker_debit: settlement.taker_debit,
                        taker_credit: settlement.taker_credit,
                        asset: settlement.asset.clone(),
                        currency: settlement.currency.clone(),
                    }),
                }),
                proof: Some(ApiProof {
                    sdl_hash: sdl_hash.clone(),
                    status: "verified".to_string(),
                }),
            },
            FillResult::Rejected { reason, .. } => Self {
                success: false,
                fill_id,
                quote_id,
                message: format!("Fill rejected: {}", reason.message()),
                error: Some(ApiFillError {
                    code: reason.code().to_string(),
                    message: reason.message(),
                    details: serde_json::to_value(reason).ok(),
                }),
                receipt: None,
                proof: None,
            },
        }
    }
}

// ============================================================================
// Get Receipts Response
// ============================================================================

/// Flattened receipt for list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiReceiptSummary {
    /// Receipt ID
    pub id: String,
    /// Quote ID
    pub quote_id: String,
    /// Whether fill was accepted
    pub success: bool,
    /// Status: "accepted" or "rejected"
    pub status: String,
    /// Taker's owner ID
    pub taker_owner_id: String,
    /// Taker's shard
    pub taker_shard: u64,
    /// Fill size
    pub size: f64,
    /// Fill price
    pub price: f64,
    /// When attempted (unix timestamp)
    pub attempted_at: i64,
    /// Error code if rejected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Error message if rejected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// SDL hash if accepted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdl_hash: Option<String>,
}

impl From<&FillReceipt> for ApiReceiptSummary {
    fn from(r: &FillReceipt) -> Self {
        let (success, status, error_code, error_message, sdl_hash) = match &r.result {
            FillResult::Accepted { sdl_hash, .. } => (
                true,
                "accepted".to_string(),
                None,
                None,
                Some(sdl_hash.clone()),
            ),
            FillResult::Rejected { reason, .. } => (
                false,
                "rejected".to_string(),
                Some(reason.code().to_string()),
                Some(reason.message()),
                None,
            ),
        };

        Self {
            id: r.receipt_id.to_string(),
            quote_id: r.quote.id.to_string(),
            success,
            status,
            taker_owner_id: r.fill_attempt.taker_owner_id.clone(),
            taker_shard: r.fill_attempt.taker_shard,
            size: r.fill_attempt.size,
            price: r.fill_attempt.price,
            attempted_at: r.fill_attempt.attempted_at.timestamp(),
            error_code,
            error_message,
            sdl_hash,
        }
    }
}
