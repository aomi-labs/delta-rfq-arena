//! Fill attempt models

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::FeedEvidence;

// UUID-dependent types only available with std feature
#[cfg(feature = "std")]
use uuid::Uuid;

#[cfg(feature = "std")]
use crate::QuoteId;

/// Unique identifier for a fill attempt
#[cfg(feature = "std")]
pub type FillId = Uuid;

/// A fill attempt by a taker
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillAttempt {
    /// Unique identifier for this fill attempt
    pub id: FillId,
    /// The quote being filled
    pub quote_id: QuoteId,
    /// The taker's owner ID
    pub taker_owner_id: String,
    /// The taker's shard
    pub taker_shard: u64,
    /// The size to fill
    pub size: f64,
    /// The price offered
    pub price: f64,
    /// Price feed evidence
    pub feed_evidence: Vec<FeedEvidence>,
    /// When the fill was attempted
    pub attempted_at: DateTime<Utc>,
}

/// The result of a fill attempt
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FillResult {
    /// Fill was accepted and settled
    Accepted {
        /// The fill ID
        fill_id: FillId,
        /// The SDL hash from delta
        sdl_hash: String,
        /// Settlement details
        settlement: SettlementDetails,
    },
    /// Fill was rejected
    Rejected {
        /// The fill ID
        fill_id: FillId,
        /// The reason for rejection
        reason: RejectionReason,
    },
}

/// Details of a successful settlement
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDetails {
    /// Amount debited from maker
    pub maker_debit: u64,
    /// Amount credited to maker
    pub maker_credit: u64,
    /// Amount debited from taker
    pub taker_debit: u64,
    /// Amount credited to taker
    pub taker_credit: u64,
    /// The asset transferred
    pub asset: String,
    /// The currency transferred
    pub currency: String,
    /// Timestamp of settlement
    pub settled_at: DateTime<Utc>,
}

/// Reason for rejecting a fill
///
/// This type is available in both std and no_std environments
/// as it's needed for validation in the zkVM.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum RejectionReason {
    /// Quote has expired
    QuoteExpired {
        expired_at: DateTime<Utc>,
        attempted_at: DateTime<Utc>,
    },
    /// Quote was already filled
    AlreadyFilled {
        filled_at: DateTime<Utc>,
    },
    /// Price feed data is stale
    StaleFeed {
        source: String,
        feed_timestamp: u64,
        current_timestamp: u64,
        max_staleness_secs: u64,
    },
    /// Price feed source not in allowlist
    UnauthorizedSource {
        source: String,
        allowed_sources: Vec<String>,
    },
    /// Taker not in allowlist
    UnauthorizedTaker {
        taker: String,
        allowed_takers: Vec<String>,
    },
    /// Price exceeds limit
    PriceExceedsLimit {
        offered_price: f64,
        limit_price: f64,
    },
    /// Size exceeds maximum
    SizeExceedsMax {
        offered_size: f64,
        max_size: f64,
    },
    /// Quorum not met (not enough sources or too much disagreement)
    QuorumNotMet {
        sources_provided: usize,
        quorum_required: u32,
        price_spread_percent: Option<f64>,
        max_tolerance_percent: f64,
    },
    /// Attempted side-payment detected
    SidePaymentDetected {
        description: String,
    },
    /// Transfer pattern invalid (not atomic DvP)
    InvalidTransferPattern {
        expected: String,
        actual: String,
    },
    /// Insufficient balance
    InsufficientBalance {
        required: u64,
        available: u64,
    },
    /// Generic validation error
    ValidationError {
        message: String,
    },
}

impl RejectionReason {
    /// Get a human-readable message for this rejection
    pub fn message(&self) -> String {
        match self {
            Self::QuoteExpired { expired_at, attempted_at } => {
                format!("Quote expired at {} (attempted at {})", expired_at, attempted_at)
            }
            Self::AlreadyFilled { filled_at } => {
                format!("Quote was already filled at {}", filled_at)
            }
            Self::StaleFeed { source, feed_timestamp, current_timestamp, max_staleness_secs } => {
                let age = current_timestamp - feed_timestamp;
                format!(
                    "Feed data from '{}' is stale: {}s old, max allowed is {}s",
                    source, age, max_staleness_secs
                )
            }
            Self::UnauthorizedSource { source, allowed_sources } => {
                format!(
                    "Source '{}' not in allowlist. Allowed: {:?}",
                    source, allowed_sources
                )
            }
            Self::UnauthorizedTaker { taker, allowed_takers } => {
                format!(
                    "Taker '{}' not in allowlist. Allowed: {:?}",
                    taker, allowed_takers
                )
            }
            Self::PriceExceedsLimit { offered_price, limit_price } => {
                format!(
                    "Offered price {} exceeds limit {}",
                    offered_price, limit_price
                )
            }
            Self::SizeExceedsMax { offered_size, max_size } => {
                format!(
                    "Offered size {} exceeds max {}",
                    offered_size, max_size
                )
            }
            Self::QuorumNotMet { sources_provided, quorum_required, price_spread_percent, max_tolerance_percent } => {
                if let Some(spread) = price_spread_percent {
                    format!(
                        "Price spread {}% exceeds tolerance {}%",
                        spread, max_tolerance_percent
                    )
                } else {
                    format!(
                        "Only {} sources provided, {} required for quorum",
                        sources_provided, quorum_required
                    )
                }
            }
            Self::SidePaymentDetected { description } => {
                format!("Side-payment detected: {}", description)
            }
            Self::InvalidTransferPattern { expected, actual } => {
                format!(
                    "Invalid transfer pattern. Expected: {}, got: {}",
                    expected, actual
                )
            }
            Self::InsufficientBalance { required, available } => {
                format!(
                    "Insufficient balance: required {}, available {}",
                    required, available
                )
            }
            Self::ValidationError { message } => message.clone(),
        }
    }

    /// Get a machine-readable error code
    pub fn code(&self) -> &'static str {
        match self {
            Self::QuoteExpired { .. } => "QUOTE_EXPIRED",
            Self::AlreadyFilled { .. } => "ALREADY_FILLED",
            Self::StaleFeed { .. } => "STALE_FEED",
            Self::UnauthorizedSource { .. } => "UNAUTHORIZED_SOURCE",
            Self::UnauthorizedTaker { .. } => "UNAUTHORIZED_TAKER",
            Self::PriceExceedsLimit { .. } => "PRICE_EXCEEDS_LIMIT",
            Self::SizeExceedsMax { .. } => "SIZE_EXCEEDS_MAX",
            Self::QuorumNotMet { .. } => "QUORUM_NOT_MET",
            Self::SidePaymentDetected { .. } => "SIDE_PAYMENT_DETECTED",
            Self::InvalidTransferPattern { .. } => "INVALID_TRANSFER_PATTERN",
            Self::InsufficientBalance { .. } => "INSUFFICIENT_BALANCE",
            Self::ValidationError { .. } => "VALIDATION_ERROR",
        }
    }
}

/// Request to attempt a fill
#[cfg(feature = "std")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillRequest {
    /// The taker's owner ID
    pub taker_owner_id: String,
    /// The taker's shard
    pub taker_shard: u64,
    /// The size to fill
    pub size: f64,
    /// The price offered
    pub price: f64,
    /// Price feed evidence
    pub feed_evidence: Vec<FeedEvidence>,
}
