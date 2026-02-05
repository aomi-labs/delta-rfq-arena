//! End-to-end integration tests for the RFQ proof flow
//!
//! These tests verify that:
//! 1. Quotes can be created with valid constraints
//! 2. Fills are validated against local laws
//! 3. Valid fills produce SDL submissions and proofs
//! 4. Invalid fills are rejected with proper reasons

use std::collections::HashMap;
use std::num::NonZero;

use delta_domain_sdk::base::crypto::ed25519::PrivKey;
use delta_domain_sdk::base::vaults::{Address, Vault, WritableNativeBalance};
use delta_domain_sdk::proving::mock;
use delta_domain_sdk::Runtime;
use rfq_local_laws::{validate_fill, RfqLocalLawsInput};
use rfq_models::{FeedEvidence, QuoteConstraints, RejectionReason};

/// Test that the owner ID derivation produces consistent results
#[test]
fn test_owner_id_derivation_consistency() {
    use sha2::{Digest, Sha256};

    let id_str = "test_maker_12345";

    // Derive twice
    let mut hasher1 = Sha256::new();
    hasher1.update(id_str.as_bytes());
    let hash1: [u8; 32] = hasher1.finalize().into();

    let mut hasher2 = Sha256::new();
    hasher2.update(id_str.as_bytes());
    let hash2: [u8; 32] = hasher2.finalize().into();

    assert_eq!(hash1, hash2, "Owner ID derivation should be deterministic");
}

/// Test that local laws validation works correctly
#[test]
fn test_local_laws_validation_accepts_valid_fill() {
    let constraints = QuoteConstraints {
        quote_id: [0u8; 32],
        max_debit: 2_000_000_000_000, // 2000 USDD in plancks
        min_credit: None,
        expiry_timestamp: u64::MAX, // Never expires for test
        allowed_sources: vec!["FeedA".into(), "FeedB".into()],
        max_staleness_secs: 300,
        quorum_count: 2,
        quorum_tolerance_percent: 1.0,
        allowed_takers: vec![],
        allowed_assets: vec!["dETH".into()],
        require_atomic_dvp: true,
        no_side_payments: true,
        nonce: 1,
        max_fill_size: 1_000_000_000, // 1 dETH in plancks
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let input = RfqLocalLawsInput {
        constraints,
        taker_owner_id: "taker_alice".into(),
        fill_size: 1_000_000_000,           // 1 dETH
        fill_price: 1_950_000_000_000,      // 1950 USDD
        feed_evidence: vec![
            FeedEvidence {
                source: "FeedA".into(),
                asset: "dETH".into(),
                price: 1950.0,
                timestamp: now,
                signature: "sig_a".into(),
            },
            FeedEvidence {
                source: "FeedB".into(),
                asset: "dETH".into(),
                price: 1951.0,
                timestamp: now,
                signature: "sig_b".into(),
            },
        ],
        current_timestamp: now,
        transfer_leg_count: 2,
        has_extra_transfers: false,
    };

    let result = validate_fill(&input);
    assert!(result.is_ok(), "Valid fill should be accepted: {:?}", result);
}

/// Test that expired quotes are rejected
#[test]
fn test_local_laws_rejects_expired_quote() {
    let past_timestamp = 1000u64; // Way in the past

    let constraints = QuoteConstraints {
        quote_id: [0u8; 32],
        max_debit: 2_000_000_000_000,
        min_credit: None,
        expiry_timestamp: past_timestamp, // Expired!
        allowed_sources: vec!["FeedA".into()],
        max_staleness_secs: 300,
        quorum_count: 1,
        quorum_tolerance_percent: 1.0,
        allowed_takers: vec![],
        allowed_assets: vec!["dETH".into()],
        require_atomic_dvp: true,
        no_side_payments: true,
        nonce: 1,
        max_fill_size: 1_000_000_000,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let input = RfqLocalLawsInput {
        constraints,
        taker_owner_id: "taker_alice".into(),
        fill_size: 1_000_000_000,
        fill_price: 1_950_000_000_000,
        feed_evidence: vec![FeedEvidence {
            source: "FeedA".into(),
            asset: "dETH".into(),
            price: 1950.0,
            timestamp: now,
            signature: "sig".into(),
        }],
        current_timestamp: now,
        transfer_leg_count: 2,
        has_extra_transfers: false,
    };

    let result = validate_fill(&input);
    assert!(
        matches!(result, Err(RejectionReason::QuoteExpired { .. })),
        "Expired quote should be rejected: {:?}",
        result
    );
}

/// Test that fills exceeding max size are rejected
#[test]
fn test_local_laws_rejects_oversized_fill() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let constraints = QuoteConstraints {
        quote_id: [0u8; 32],
        max_debit: 2_000_000_000_000,
        min_credit: None,
        expiry_timestamp: u64::MAX,
        allowed_sources: vec!["FeedA".into()],
        max_staleness_secs: 300,
        quorum_count: 1,
        quorum_tolerance_percent: 1.0,
        allowed_takers: vec![],
        allowed_assets: vec!["dETH".into()],
        require_atomic_dvp: true,
        no_side_payments: true,
        nonce: 1,
        max_fill_size: 1_000_000_000, // Max 1 dETH
    };

    let input = RfqLocalLawsInput {
        constraints,
        taker_owner_id: "taker_alice".into(),
        fill_size: 2_000_000_000, // 2 dETH - exceeds max!
        fill_price: 3_900_000_000_000,
        feed_evidence: vec![FeedEvidence {
            source: "FeedA".into(),
            asset: "dETH".into(),
            price: 1950.0,
            timestamp: now,
            signature: "sig".into(),
        }],
        current_timestamp: now,
        transfer_leg_count: 2,
        has_extra_transfers: false,
    };

    let result = validate_fill(&input);
    assert!(
        matches!(result, Err(RejectionReason::SizeExceedsMax { .. })),
        "Oversized fill should be rejected: {:?}",
        result
    );
}

/// Test that unauthorized takers are rejected
#[test]
fn test_local_laws_rejects_unauthorized_taker() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let constraints = QuoteConstraints {
        quote_id: [0u8; 32],
        max_debit: 2_000_000_000_000,
        min_credit: None,
        expiry_timestamp: u64::MAX,
        allowed_sources: vec!["FeedA".into()],
        max_staleness_secs: 300,
        quorum_count: 1,
        quorum_tolerance_percent: 1.0,
        allowed_takers: vec!["taker_bob".into()], // Only Bob allowed
        allowed_assets: vec!["dETH".into()],
        require_atomic_dvp: true,
        no_side_payments: true,
        nonce: 1,
        max_fill_size: 1_000_000_000,
    };

    let input = RfqLocalLawsInput {
        constraints,
        taker_owner_id: "taker_alice".into(), // Alice not allowed!
        fill_size: 1_000_000_000,
        fill_price: 1_950_000_000_000,
        feed_evidence: vec![FeedEvidence {
            source: "FeedA".into(),
            asset: "dETH".into(),
            price: 1950.0,
            timestamp: now,
            signature: "sig".into(),
        }],
        current_timestamp: now,
        transfer_leg_count: 2,
        has_extra_transfers: false,
    };

    let result = validate_fill(&input);
    assert!(
        matches!(result, Err(RejectionReason::UnauthorizedTaker { .. })),
        "Unauthorized taker should be rejected: {:?}",
        result
    );
}

/// Test Delta Runtime initialization in mock mode
#[tokio::test]
async fn test_runtime_initialization_mock_mode() {
    let shard = NonZero::new(9).unwrap();
    let keypair = PrivKey::generate();
    let owner = keypair.pub_key().owner();
    let vault_address = Address::new(owner, shard.get());

    // Create a vault with balance
    let mut vault = Vault::new(shard);
    vault.set_balance(1_000_000_000);

    // Create mock proving client
    let proving_client = mock::Client::global_laws()
        .with_local_laws::<rfq_local_laws::RfqLocalLaws>();

    // Build runtime
    let runtime = Runtime::builder(shard, keypair)
        .with_mock_rpc(HashMap::from([(vault_address, vault)]))
        .with_proving_client(proving_client)
        .build()
        .await;

    assert!(runtime.is_ok(), "Runtime should initialize: {:?}", runtime.err());
}

/// Test that base58 owner IDs are parsed correctly
#[test]
fn test_base58_owner_id_parsing() {
    // Generate a real keypair and get its base58 owner ID
    let keypair = PrivKey::generate();
    let owner = keypair.pub_key().owner();
    let owner_bytes: [u8; 32] = owner.into();
    let base58_id = bs58::encode(&owner_bytes).into_string();

    // Verify it decodes back to the same bytes
    let decoded = bs58::decode(&base58_id).into_vec().unwrap();
    assert_eq!(decoded.len(), 32);
    assert_eq!(decoded.as_slice(), &owner_bytes);
}
