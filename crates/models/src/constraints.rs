//! Quote constraints (guardrails) compiled from English
//!
//! These constraints are enforced by Local Laws during settlement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The constraints (guardrails) for a quote
///
/// These are compiled from the maker's English quote text
/// and enforced at settlement time by Local Laws.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteConstraints {
    /// Unique identifier linking to the quote
    pub quote_id: [u8; 32],

    /// Maximum amount that can be debited (in smallest units)
    /// For a buy order, this is the max price * size
    pub max_debit: u64,

    /// Minimum amount that must be received (in smallest units)
    /// For a sell order, this is the min price * size
    pub min_credit: Option<u64>,

    /// When the quote expires (Unix timestamp)
    pub expiry_timestamp: u64,

    /// Allowed price feed sources (e.g., ["FeedA", "FeedB"])
    pub allowed_sources: Vec<String>,

    /// Maximum age of price feed data in seconds
    pub max_staleness_secs: u64,

    /// Minimum number of sources that must agree (quorum)
    pub quorum_count: u32,

    /// Maximum percentage difference between sources for quorum
    pub quorum_tolerance_percent: f64,

    /// Allowed taker owner IDs (empty = any taker allowed)
    pub allowed_takers: Vec<String>,

    /// Allowed asset token IDs that can be transferred
    pub allowed_assets: Vec<String>,

    /// Whether atomic DvP (delivery vs payment) is required
    /// If true, the fill must be a single atomic transaction
    pub require_atomic_dvp: bool,

    /// Whether side-payments are prohibited
    /// If true, only the main asset exchange is allowed
    pub no_side_payments: bool,

    /// The nonce for replay protection (quote can only be filled once)
    pub nonce: u64,

    /// Maximum size that can be filled
    pub max_fill_size: u64,
}

impl QuoteConstraints {
    /// Create a new QuoteConstraints with sensible defaults
    pub fn new(quote_id: [u8; 32]) -> Self {
        Self {
            quote_id,
            max_debit: 0,
            min_credit: None,
            expiry_timestamp: 0,
            allowed_sources: vec![],
            max_staleness_secs: 60, // 1 minute default
            quorum_count: 1,
            quorum_tolerance_percent: 1.0,
            allowed_takers: vec![],
            allowed_assets: vec![],
            require_atomic_dvp: true,
            no_side_payments: true,
            nonce: 0,
            max_fill_size: 0,
        }
    }

    /// Convert expiry timestamp to DateTime
    pub fn expiry_datetime(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.expiry_timestamp as i64, 0)
            .unwrap_or_else(|| Utc::now())
    }

    /// Check if the constraints allow a specific taker
    pub fn allows_taker(&self, taker_owner_id: &str) -> bool {
        self.allowed_takers.is_empty() || self.allowed_takers.contains(&taker_owner_id.to_string())
    }

    /// Check if the constraints allow a specific source
    pub fn allows_source(&self, source: &str) -> bool {
        self.allowed_sources.is_empty() || self.allowed_sources.contains(&source.to_string())
    }
}

/// Evidence from a price feed, included with a fill attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEvidence {
    /// The source name (e.g., "FeedA")
    pub source: String,
    /// The asset being priced
    pub asset: String,
    /// The price
    pub price: f64,
    /// Unix timestamp when the price was fetched
    pub timestamp: u64,
    /// Signature from the feed (for verification)
    pub signature: String,
}

impl FeedEvidence {
    /// Check if this evidence is fresh enough given max staleness
    pub fn is_fresh(&self, max_staleness_secs: u64, current_time: u64) -> bool {
        current_time.saturating_sub(self.timestamp) <= max_staleness_secs
    }
}
