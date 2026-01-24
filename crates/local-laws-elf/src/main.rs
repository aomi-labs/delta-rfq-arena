//! RFQ Local Laws ELF Program
//!
//! This program is compiled to RISC-V and executed inside the SP1 zkVM.
//! It validates fill attempts against quote constraints (guardrails)
//! and produces a ZK proof that the validation was performed correctly.
//!
//! Note: We use our own types here instead of delta SDK types to avoid
//! zkVM-incompatible dependencies.

#![no_main]
sp1_zkvm::entrypoint!(main);

use rfq_local_laws::RfqLocalLawsInput;

/// Main entry point for the ZK prover
///
/// This function:
/// 1. Reads the RfqLocalLawsInput from the prover
/// 2. Validates the fill against constraints
/// 3. Commits the quote_id as public output
pub fn main() {
    // Read the local laws input
    let input: RfqLocalLawsInput = sp1_zkvm::io::read();

    // Validate using our standalone validation function
    // This will panic if validation fails, causing the proof to fail
    rfq_local_laws::validate_fill(&input)
        .expect("Local laws validation failed");

    // Commit the quote_id as public output
    // This allows verifiers to know which quote was validated
    sp1_zkvm::io::commit_slice(&input.constraints.quote_id);
    
    // Commit success
    sp1_zkvm::io::commit(&1u8);
}
