//! Example: How Delta Local Laws Work
//!
//! This file demonstrates the complete flow of defining, compiling, and using
//! local laws on the delta network.
//!
//! Run with: cargo run -p rfq-domain --example local_laws_demo

use delta_local_laws::{LocalLaws, LocalLawsError};
use delta_serializers::{bytes::BytesSerializer, serializer::Serializer};
use delta_verifiable::types::{VerifiableWithDiffs, VerificationContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// STEP 1: Define Your Local Laws Input Structure
// ============================================================================
//
// This is the "guardrails" data that gets passed during proof generation.
// It can contain any serializable data you need to validate transactions.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RfqGuardrails {
    /// Maximum amount that can be debited
    pub max_debit: u64,
    /// Quote expiry timestamp
    pub expiry_timestamp: u64,
    /// Allowed price feed sources
    pub allowed_sources: Vec<String>,
    /// Maximum staleness of feed data (seconds)
    pub max_staleness_secs: u64,
    /// Allowed taker addresses (empty = any)
    pub allowed_takers: Vec<String>,
    /// Current timestamp for validation
    pub current_timestamp: u64,
    /// Taker attempting the fill
    pub taker_id: String,
    /// Amount being transferred
    pub transfer_amount: u64,
}

// ============================================================================
// STEP 2: Implement the LocalLaws Trait
// ============================================================================
//
// This is where the actual validation logic lives. Every fill attempt will
// be checked against these rules during proof generation.

#[derive(Debug, Clone, Copy, Default)]
pub struct RfqLocalLaws;

impl LocalLaws for RfqLocalLaws {
    // The Input type must be Serialize + Deserialize
    type Input<'a> = RfqGuardrails;

    fn validate<'a>(
        _verifiables: &[VerifiableWithDiffs],
        _verification_context: &VerificationContext,
        input: &RfqGuardrails,
    ) -> Result<(), LocalLawsError> {
        // 1. Check expiry
        if input.current_timestamp > input.expiry_timestamp {
            return Err(LocalLawsError::new(format!(
                "Quote expired at {}, current time is {}",
                input.expiry_timestamp, input.current_timestamp
            )));
        }

        // 2. Check max debit
        if input.transfer_amount > input.max_debit {
            return Err(LocalLawsError::new(format!(
                "Transfer amount {} exceeds max debit {}",
                input.transfer_amount, input.max_debit
            )));
        }

        // 3. Check taker allowlist
        if !input.allowed_takers.is_empty() && !input.allowed_takers.contains(&input.taker_id) {
            return Err(LocalLawsError::new(format!(
                "Taker '{}' not in allowlist: {:?}",
                input.taker_id, input.allowed_takers
            )));
        }

        println!("  [OK] All guardrails satisfied!");
        Ok(())
    }
}

// ============================================================================
// STEP 3: Test the Local Laws
// ============================================================================

fn main() {
    println!("\n========================================");
    println!("   Delta Local Laws Demo");
    println!("========================================\n");

    // Create empty verification context (we don't need vaults for this demo)
    let empty_context = VerificationContext {
        vaults: HashMap::new(),
        shard: 1,
    };

    // Create base guardrails
    let base_guardrails = RfqGuardrails {
        max_debit: 2000,                // Max 2000 units
        expiry_timestamp: 1737500000,   // Some future timestamp
        allowed_sources: vec!["FeedA".to_string(), "FeedB".to_string()],
        max_staleness_secs: 5,
        allowed_takers: vec![],         // Any taker allowed
        current_timestamp: 1737499990,  // 10 seconds before expiry
        taker_id: "taker_123".to_string(),
        transfer_amount: 1500,
    };

    // Test 1: Valid fill (all constraints satisfied)
    println!("Test 1: Valid fill (amount=1500, max=2000, not expired)");
    let result = RfqLocalLaws::validate(
        &[],
        &empty_context,
        &base_guardrails,
    );
    match &result {
        Ok(()) => println!("  Result: ACCEPTED\n"),
        Err(e) => println!("  Result: REJECTED - {}\n", e),
    }

    // Test 2: Expired quote
    println!("Test 2: Expired quote");
    let expired = RfqGuardrails {
        current_timestamp: 1737500001,  // After expiry
        ..base_guardrails.clone()
    };
    let result = RfqLocalLaws::validate(&[], &empty_context, &expired);
    match &result {
        Ok(()) => println!("  Result: ACCEPTED\n"),
        Err(e) => println!("  Result: REJECTED - {}\n", e),
    }

    // Test 3: Amount exceeds max debit
    println!("Test 3: Amount exceeds max debit (amount=2500, max=2000)");
    let over_limit = RfqGuardrails {
        transfer_amount: 2500,
        ..base_guardrails.clone()
    };
    let result = RfqLocalLaws::validate(&[], &empty_context, &over_limit);
    match &result {
        Ok(()) => println!("  Result: ACCEPTED\n"),
        Err(e) => println!("  Result: REJECTED - {}\n", e),
    }

    // Test 4: Unauthorized taker
    println!("Test 4: Unauthorized taker");
    let unauthorized = RfqGuardrails {
        allowed_takers: vec!["approved_taker_1".to_string(), "approved_taker_2".to_string()],
        taker_id: "malicious_taker".to_string(),
        ..base_guardrails.clone()
    };
    let result = RfqLocalLaws::validate(&[], &empty_context, &unauthorized);
    match &result {
        Ok(()) => println!("  Result: ACCEPTED\n"),
        Err(e) => println!("  Result: REJECTED - {}\n", e),
    }

    // Test 5: Serialize guardrails (how they're passed to prover)
    println!("Test 5: Serialization for proof generation");
    let serialized = BytesSerializer::serialize(&base_guardrails).unwrap();
    println!("  Serialized guardrails: {} bytes", serialized.len());
    let deserialized: RfqGuardrails = BytesSerializer::deserialize(&serialized).unwrap();
    println!("  Deserialized max_debit: {}", deserialized.max_debit);
    println!("  Round-trip successful!\n");

    // Summary
    println!("========================================");
    println!("   Summary: How Local Laws Work");
    println!("========================================\n");
    println!("1. DEFINE: Create a struct for your guardrails (RfqGuardrails)");
    println!("2. IMPLEMENT: Implement LocalLaws trait with validation logic");
    println!("3. SERIALIZE: Guardrails are serialized when passed to prover");
    println!("4. VALIDATE: During proof generation, validate() is called");
    println!("5. REJECT: If any check fails, transaction cannot settle\n");
    
    println!("For production ZK proofs:");
    println!("- Create a separate crate with #![no_main]");
    println!("- Use sp1_zkvm::entrypoint!(main)");
    println!("- Compile to ELF binary");
    println!("- Use with Runtime::builder().with_proving_client(sp1::Client...)");
}
