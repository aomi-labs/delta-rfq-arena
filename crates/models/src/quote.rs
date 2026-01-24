//! Quote models for the RFQ system

#![allow(unused_imports)]

use alloc::string::String;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
use uuid::Uuid;

#[cfg(feature = "std")]
use crate::QuoteConstraints;

/// Unique identifier for a quote (only available with std)
#[cfg(feature = "std")]
pub type QuoteId = Uuid;

/// The side of a trade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

/// The status of a quote
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteStatus {
    /// Quote is active and can be filled
    Active,
    /// Quote has been filled
    Filled,
    /// Quote has expired
    Expired,
    /// Quote was cancelled by maker
    Cancelled,
}

/// The specification of a quote (what the maker wants to trade)
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteSpec {
    /// The asset being traded (e.g., "dETH")
    pub asset: String,
    /// The size of the trade
    pub size: f64,
    /// Buy or Sell
    pub side: Side,
    /// Maximum price (for buys) or minimum price (for sells)
    pub limit_price: Option<f64>,
    /// The currency for settlement (e.g., "USDD")
    pub currency: String,
}

/// A complete quote posted by a maker
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    /// Unique identifier
    pub id: QuoteId,
    /// The quote specification
    pub spec: QuoteSpec,
    /// The guardrails (constraints) for this quote
    pub constraints: QuoteConstraints,
    /// Current status
    pub status: QuoteStatus,
    /// When the quote was created
    pub created_at: DateTime<Utc>,
    /// When the quote expires
    pub expires_at: DateTime<Utc>,
    /// The maker's owner ID (base58 encoded)
    pub maker_owner_id: String,
    /// The maker's vault address on delta
    pub maker_vault_address: String,
    /// Original English text (for display)
    pub original_text: String,
}

#[cfg(feature = "std")]
impl Quote {
    /// Check if the quote is still valid (not expired, not filled, not cancelled)
    pub fn is_active(&self) -> bool {
        self.status == QuoteStatus::Active && Utc::now() < self.expires_at
    }

    /// Check if the quote has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

/// Request to create a new quote
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuoteRequest {
    /// The English text describing the quote
    pub text: String,
    /// The maker's owner ID
    pub maker_owner_id: String,
    /// The maker's shard
    pub maker_shard: u64,
}

/// Response after creating a quote
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuoteResponse {
    /// The created quote
    pub quote: Quote,
    /// Human-readable summary of the compiled constraints
    pub constraints_summary: String,
}
