//! Local Laws for the OTC RFQ Arena
//!
//! This crate implements the guardrails that are enforced at settlement time.
//! These rules are compiled from the maker's English quote and validated
//! during proof generation.
//!
//! ## Features
//!
//! - `std` (default): Standard library support
//! - `delta-sdk` (default): Full integration with delta SDK types
//! - `zkvm`: Minimal build for SP1 zkVM proving

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "delta-sdk")]
use delta_local_laws::{LocalLaws, LocalLawsError};
#[cfg(feature = "delta-sdk")]
use delta_verifiable::types::{VerifiableWithDiffs, VerificationContext};

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use rfq_models::{FeedEvidence, QuoteConstraints, RejectionReason};
use serde::{Deserialize, Serialize};

/// Input to the RFQ Local Laws
///
/// This is passed during proof generation and contains all the
/// information needed to validate a fill against the quote constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfqLocalLawsInput {
    /// The quote constraints (guardrails)
    pub constraints: QuoteConstraints,
    /// The taker's owner ID
    pub taker_owner_id: String,
    /// The fill size
    pub fill_size: u64,
    /// The fill price (in smallest units)
    pub fill_price: u64,
    /// Price feed evidence
    pub feed_evidence: Vec<FeedEvidence>,
    /// Current timestamp (for expiry and staleness checks)
    pub current_timestamp: u64,
    /// Number of transfer legs in the transaction
    pub transfer_leg_count: usize,
    /// Whether there are any transfers outside the expected pattern
    pub has_extra_transfers: bool,
}

/// RFQ Local Laws implementation
///
/// Validates fill attempts against quote constraints (guardrails)
#[derive(Debug, Clone, Copy, Default)]
pub struct RfqLocalLaws;

// Delta SDK integration - only compiled when delta-sdk feature is enabled
#[cfg(feature = "delta-sdk")]
impl LocalLaws for RfqLocalLaws {
    type Input<'a> = RfqLocalLawsInput;

    fn validate<'a>(
        _verifiables: &[VerifiableWithDiffs],
        _verification_context: &VerificationContext,
        input: &RfqLocalLawsInput,
    ) -> Result<(), LocalLawsError> {
        validate_fill_internal(input).map_err(|e| LocalLawsError::new(e.message()))
    }
}

/// Validate a fill attempt and return a detailed rejection reason if invalid
///
/// This is the core validation logic that works in both std and zkVM environments.
pub fn validate_fill(input: &RfqLocalLawsInput) -> Result<(), RejectionReason> {
    validate_fill_internal(input)
}

/// Internal validation logic
fn validate_fill_internal(input: &RfqLocalLawsInput) -> Result<(), RejectionReason> {
    let constraints = &input.constraints;

    // 1. Check expiry
    if input.current_timestamp > constraints.expiry_timestamp {
        return Err(RejectionReason::QuoteExpired {
            expired_at: constraints.expiry_datetime(),
            attempted_at: chrono::Utc::now(),
        });
    }

    // 2. Check taker allowlist
    if !constraints.allowed_takers.is_empty()
        && !constraints.allowed_takers.contains(&input.taker_owner_id)
    {
        return Err(RejectionReason::UnauthorizedTaker {
            taker: input.taker_owner_id.clone(),
            allowed_takers: constraints.allowed_takers.clone(),
        });
    }

    // 3. Check fill size
    if input.fill_size > constraints.max_fill_size {
        return Err(RejectionReason::SizeExceedsMax {
            offered_size: input.fill_size as f64,
            max_size: constraints.max_fill_size as f64,
        });
    }

    // 4. Check max debit
    if input.fill_price > constraints.max_debit {
        return Err(RejectionReason::PriceExceedsLimit {
            offered_price: input.fill_price as f64,
            limit_price: constraints.max_debit as f64,
        });
    }

    // 5. Validate feed evidence
    validate_feed_evidence_detailed(input)?;

    // 6. Check transfer pattern
    if constraints.require_atomic_dvp && input.transfer_leg_count != 2 {
        return Err(RejectionReason::InvalidTransferPattern {
            expected: String::from("2 legs (atomic DvP)"),
            actual: format!("{} legs", input.transfer_leg_count),
        });
    }

    // 7. Check for side-payments
    if constraints.no_side_payments && input.has_extra_transfers {
        return Err(RejectionReason::SidePaymentDetected {
            description: String::from("Extra transfers detected outside expected pattern"),
        });
    }

    Ok(())
}

/// Validate feed evidence with detailed rejection reasons
fn validate_feed_evidence_detailed(input: &RfqLocalLawsInput) -> Result<(), RejectionReason> {
    let constraints = &input.constraints;

    // Check quorum count
    if input.feed_evidence.len() < constraints.quorum_count as usize {
        return Err(RejectionReason::QuorumNotMet {
            sources_provided: input.feed_evidence.len(),
            quorum_required: constraints.quorum_count,
            price_spread_percent: None,
            max_tolerance_percent: constraints.quorum_tolerance_percent,
        });
    }

    let mut valid_prices: Vec<f64> = Vec::new();

    for evidence in &input.feed_evidence {
        // Check source allowlist
        if !constraints.allowed_sources.is_empty()
            && !constraints.allowed_sources.contains(&evidence.source)
        {
            return Err(RejectionReason::UnauthorizedSource {
                source: evidence.source.clone(),
                allowed_sources: constraints.allowed_sources.clone(),
            });
        }

        // Check freshness
        let age = input.current_timestamp.saturating_sub(evidence.timestamp);
        if age > constraints.max_staleness_secs {
            return Err(RejectionReason::StaleFeed {
                source: evidence.source.clone(),
                feed_timestamp: evidence.timestamp,
                current_timestamp: input.current_timestamp,
                max_staleness_secs: constraints.max_staleness_secs,
            });
        }

        valid_prices.push(evidence.price);
    }

    // Check price quorum
    if valid_prices.len() >= 2 {
        let min_price = valid_prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = valid_prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        if min_price > 0.0 {
            let spread_percent = ((max_price - min_price) / min_price) * 100.0;
            if spread_percent > constraints.quorum_tolerance_percent {
                return Err(RejectionReason::QuorumNotMet {
                    sources_provided: valid_prices.len(),
                    quorum_required: constraints.quorum_count,
                    price_spread_percent: Some(spread_percent),
                    max_tolerance_percent: constraints.quorum_tolerance_percent,
                });
            }
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "delta-sdk"))]
mod tests {
    use super::*;

    fn test_constraints() -> QuoteConstraints {
        QuoteConstraints {
            quote_id: [0u8; 32],
            max_debit: 2_000_000_000, // 2000 USDD
            min_credit: None,
            expiry_timestamp: 1737500000,
            allowed_sources: alloc::vec!["FeedA".into(), "FeedB".into()],
            max_staleness_secs: 5,
            quorum_count: 2,
            quorum_tolerance_percent: 0.5,
            allowed_takers: alloc::vec![],
            allowed_assets: alloc::vec!["dETH".into()],
            require_atomic_dvp: true,
            no_side_payments: true,
            nonce: 1,
            max_fill_size: 1_000_000_000, // 1 dETH
        }
    }

    #[test]
    fn test_valid_fill() {
        let input = RfqLocalLawsInput {
            constraints: test_constraints(),
            taker_owner_id: "some_taker".into(),
            fill_size: 1_000_000_000,
            fill_price: 1_950_000_000,
            feed_evidence: alloc::vec![
                FeedEvidence {
                    source: "FeedA".into(),
                    asset: "dETH".into(),
                    price: 1950.0,
                    timestamp: 1737499998,
                    signature: "sig".into(),
                },
                FeedEvidence {
                    source: "FeedB".into(),
                    asset: "dETH".into(),
                    price: 1951.0,
                    timestamp: 1737499999,
                    signature: "sig".into(),
                },
            ],
            current_timestamp: 1737500000,
            transfer_leg_count: 2,
            has_extra_transfers: false,
        };

        let result = validate_fill(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stale_feed_rejection() {
        let input = RfqLocalLawsInput {
            constraints: test_constraints(),
            taker_owner_id: "some_taker".into(),
            fill_size: 1_000_000_000,
            fill_price: 1_950_000_000,
            feed_evidence: alloc::vec![
                FeedEvidence {
                    source: "FeedA".into(),
                    asset: "dETH".into(),
                    price: 1950.0,
                    timestamp: 1737499990, // 10 seconds old
                    signature: "sig".into(),
                },
                FeedEvidence {
                    source: "FeedB".into(),
                    asset: "dETH".into(),
                    price: 1951.0,
                    timestamp: 1737499999,
                    signature: "sig".into(),
                },
            ],
            current_timestamp: 1737500000,
            transfer_leg_count: 2,
            has_extra_transfers: false,
        };

        let result = validate_fill(&input);
        assert!(matches!(result, Err(RejectionReason::StaleFeed { .. })));
    }

    #[test]
    fn test_unauthorized_source_rejection() {
        let input = RfqLocalLawsInput {
            constraints: test_constraints(),
            taker_owner_id: "some_taker".into(),
            fill_size: 1_000_000_000,
            fill_price: 1_950_000_000,
            feed_evidence: alloc::vec![
                FeedEvidence {
                    source: "FeedMallory".into(), // Not in allowlist
                    asset: "dETH".into(),
                    price: 1950.0,
                    timestamp: 1737499999,
                    signature: "sig".into(),
                },
                FeedEvidence {
                    source: "FeedB".into(),
                    asset: "dETH".into(),
                    price: 1951.0,
                    timestamp: 1737499999,
                    signature: "sig".into(),
                },
            ],
            current_timestamp: 1737500000,
            transfer_leg_count: 2,
            has_extra_transfers: false,
        };

        let result = validate_fill(&input);
        assert!(matches!(result, Err(RejectionReason::UnauthorizedSource { .. })));
    }
}
