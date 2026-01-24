//! Price feed models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A price update from a feed source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    /// The source name (e.g., "FeedA", "FeedB")
    pub source: String,
    /// The asset being priced (e.g., "dETH")
    pub asset: String,
    /// The price in the quote currency
    pub price: f64,
    /// The quote currency (e.g., "USDD")
    pub currency: String,
    /// Unix timestamp
    pub timestamp: u64,
    /// DateTime representation
    pub datetime: DateTime<Utc>,
    /// Signature for verification
    pub signature: String,
}

/// Configuration for a mock feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    /// The source name
    pub name: String,
    /// Base price (will be varied slightly)
    pub base_price: f64,
    /// Price variance (percentage)
    pub variance_percent: f64,
    /// Whether to return stale data
    pub force_stale: bool,
    /// If stale, how many seconds old
    pub stale_seconds: u64,
    /// Whether this is a malicious feed
    pub is_malicious: bool,
    /// If malicious, price manipulation factor
    pub manipulation_factor: f64,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            name: "FeedA".to_string(),
            base_price: 2000.0,
            variance_percent: 0.1,
            force_stale: false,
            stale_seconds: 0,
            is_malicious: false,
            manipulation_factor: 1.0,
        }
    }
}

impl FeedConfig {
    /// Create a good-faith feed config
    pub fn good(name: &str, base_price: f64) -> Self {
        Self {
            name: name.to_string(),
            base_price,
            ..Default::default()
        }
    }

    /// Create a stale feed config
    pub fn stale(name: &str, base_price: f64, stale_seconds: u64) -> Self {
        Self {
            name: name.to_string(),
            base_price,
            force_stale: true,
            stale_seconds,
            ..Default::default()
        }
    }

    /// Create a malicious feed config
    pub fn malicious(name: &str, base_price: f64, manipulation_factor: f64) -> Self {
        Self {
            name: name.to_string(),
            base_price,
            is_malicious: true,
            manipulation_factor,
            ..Default::default()
        }
    }
}
