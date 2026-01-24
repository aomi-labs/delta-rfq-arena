//! Receipt models for fill attempts
//!
//! Receipts provide a verifiable record of what happened
//! during a fill attempt, including the constraints that
//! were in force and the outcome.
//!
//! These are only used in the domain server, not in zkVM validation.

use crate::{FillAttempt, FillResult, Quote, QuoteConstraints, RejectionReason};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A receipt for a fill attempt
///
/// This provides a complete, verifiable record of:
/// - The quote and its constraints
/// - The fill attempt details
/// - The outcome (accepted or rejected)
/// - If rejected, the specific reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillReceipt {
    /// Unique receipt ID
    pub receipt_id: Uuid,
    /// The quote that was attempted to be filled
    pub quote: Quote,
    /// The constraints that were in force
    pub constraints: QuoteConstraints,
    /// The fill attempt
    pub fill_attempt: FillAttempt,
    /// The result
    pub result: FillResult,
    /// When the receipt was generated
    pub generated_at: DateTime<Utc>,
}

impl FillReceipt {
    /// Create a new receipt
    pub fn new(
        quote: Quote,
        constraints: QuoteConstraints,
        fill_attempt: FillAttempt,
        result: FillResult,
    ) -> Self {
        Self {
            receipt_id: Uuid::new_v4(),
            quote,
            constraints,
            fill_attempt,
            result,
            generated_at: Utc::now(),
        }
    }

    /// Check if the fill was accepted
    pub fn is_accepted(&self) -> bool {
        matches!(self.result, FillResult::Accepted { .. })
    }

    /// Get the rejection reason if rejected
    pub fn rejection_reason(&self) -> Option<&RejectionReason> {
        match &self.result {
            FillResult::Rejected { reason, .. } => Some(reason),
            _ => None,
        }
    }

    /// Get a summary suitable for display
    pub fn summary(&self) -> ReceiptSummary {
        ReceiptSummary {
            receipt_id: self.receipt_id,
            quote_id: self.quote.id,
            status: if self.is_accepted() { "ACCEPTED" } else { "REJECTED" }.to_string(),
            reason: self.rejection_reason().map(|r| r.message()),
            reason_code: self.rejection_reason().map(|r| r.code().to_string()),
            taker: self.fill_attempt.taker_owner_id.clone(),
            size: self.fill_attempt.size,
            price: self.fill_attempt.price,
            timestamp: self.generated_at,
        }
    }
}

/// A summary of a receipt for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSummary {
    pub receipt_id: Uuid,
    pub quote_id: Uuid,
    pub status: String,
    pub reason: Option<String>,
    pub reason_code: Option<String>,
    pub taker: String,
    pub size: f64,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
}
