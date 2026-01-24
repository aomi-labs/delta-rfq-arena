# Testnet Integration - Next Steps

This document outlines the requirements and steps needed to wire the RFQ Arena to Delta's testnet for real ZK-proven settlement.

## Current State

The domain server currently runs in **mock mode**:
- LocalLaws validation happens in-memory
- No actual ZK proofs are generated
- No on-chain settlement occurs
- Receipts are simulated

## Target State

Full integration with Delta testnet:
- LocalLaws compiled to SP1 ELF binary
- ZK proofs generated for each fill
- Settlement transactions submitted to testnet
- Cryptographic receipts with proof hashes

---

## Prerequisites

### 1. Testnet Access

| Item | Value | Status |
|------|-------|--------|
| Shard ID | `9` | Configured |
| RPC URL | `http://164.92.69.96:9000` | Configured |
| Keypair | `keypair_9.json` | Available |
| Cargo Registry Token | `HqTbxnVJLvMDAV6ktqZDxgJHjD8pwSlJ` | Configured |

### 2. SDK Dependencies

```toml
# Already in Cargo.toml
[dependencies]
delta_base_sdk = "0.5"
delta_domain_sdk = "0.5"
delta_local_laws = "0.5"
```

### 3. SP1 Toolchain (for ZK proofs)

```bash
# Install SP1
curl -L https://sp1.succinct.xyz | bash
sp1up

# Verify installation
cargo prove --version
```

> **Note (ARM Mac)**: The SP1 toolchain does not have pre-built binaries for ARM (Apple Silicon) Macs. Building requires either:
> - An x86_64 Linux/Mac machine
> - Docker with x86 emulation
> - GitHub Actions CI (recommended)
>
> The ELF crate is ready at `crates/local-laws-elf/` - just needs `cargo prove build` on a supported platform.

---

## Integration Steps

### Step 1: Create LocalLaws ELF Binary

Create a separate crate for the provable program:

```
crates/
└── local-laws-elf/
    ├── Cargo.toml
    └── src/
        └── main.rs
```

**`Cargo.toml`**:
```toml
[package]
name = "rfq-local-laws-elf"
version = "0.1.0"
edition = "2021"

[dependencies]
rfq-local-laws = { path = "../local-laws" }
rfq-models = { path = "../models" }
sp1-zkvm = "1.0"
bincode = "2.0"
```

**`src/main.rs`**:
```rust
#![no_main]
sp1_zkvm::entrypoint!(main);

use rfq_local_laws::{RfqLocalLaws, RfqLocalLawsInput};
use delta_local_laws::LocalLaws;

pub fn main() {
    // Read input from prover
    let input: RfqLocalLawsInput = sp1_zkvm::io::read();
    
    // Validate (will panic if invalid, causing proof to fail)
    RfqLocalLaws::validate_standalone(&input)
        .expect("LocalLaws validation failed");
    
    // Commit the validated input hash as public output
    let input_hash = sp1_zkvm::io::commit(&input);
}
```

**Build the ELF**:
```bash
cd crates/local-laws-elf
cargo prove build
# Output: target/elf/riscv32im-succinct-zkvm-elf
```

### Step 2: Wire Runtime in Domain Server

Update `crates/domain/src/main.rs`:

```rust
use delta_domain_sdk::{Runtime, RuntimeBuilder};
use delta_base_sdk::crypto::ed25519::Keypair;
use std::fs;

async fn create_runtime(config: &DomainConfig) -> Result<Runtime> {
    // Load keypair
    let keypair_json = fs::read_to_string(&config.keypair_path)?;
    let keypair: Keypair = serde_json::from_str(&keypair_json)?;
    
    // Load ELF binary
    let elf_bytes = include_bytes!("../../local-laws-elf/target/elf/...");
    
    // Build runtime
    let runtime = Runtime::builder(config.shard, keypair)
        .with_rpc_url(&config.rpc_url)
        .with_local_laws_elf::<RfqLocalLaws>(elf_bytes)
        .build()
        .await?;
    
    Ok(runtime)
}
```

### Step 3: Submit Fills via Runtime

Replace mock settlement with actual submission:

```rust
async fn settle_fill(
    runtime: &Runtime,
    quote: &Quote,
    fill: &FillAttempt,
    input: &RfqLocalLawsInput,
) -> Result<SettlementResult> {
    // Create the SDL (Settlement Definition Language)
    let sdl = runtime.create_sdl()
        .transfer(
            &quote.maker_vault_address,
            &fill.taker_vault_address,
            &quote.spec.currency,
            input.fill_price,
        )
        .transfer(
            &fill.taker_vault_address,
            &quote.maker_vault_address,
            &quote.spec.asset,
            input.fill_size,
        )
        .with_local_laws_input(input)
        .build()?;
    
    // Submit and wait for proof + settlement
    let result = runtime.submit_and_prove(sdl).await?;
    
    Ok(SettlementResult {
        sdl_hash: result.sdl_hash,
        proof_hash: result.proof_hash,
        block_height: result.block_height,
    })
}
```

### Step 4: Handle Async Proof Generation

Proofs take time (~30s-2min). Options:

**Option A: Synchronous (simple, slow UX)**
```rust
let result = runtime.submit_and_prove(sdl).await?; // Blocks
```

**Option B: Async with polling (better UX)**
```rust
// Submit
let submission_id = runtime.submit(sdl).await?;

// Return pending status immediately
return Ok(FillReceipt {
    status: FillStatus::Pending { submission_id },
    ..
});

// Later, poll for completion
let status = runtime.get_submission_status(submission_id).await?;
```

**Option C: Webhooks (best UX)**
```rust
runtime.submit_with_webhook(sdl, "https://your-domain/webhooks/settlement").await?;
```

---

## Configuration Requirements

### Environment Variables

```bash
# Required for testnet
SHARD=9
KEYPAIR_PATH=./keypair_9.json
RPC_URL=http://164.92.69.96:9000

# Optional: Use mock for local dev
USE_MOCK_RUNTIME=1

# SP1 proving (if self-hosted)
SP1_PROVER=network  # or "local" for CPU proving
SP1_PRIVATE_KEY=... # For network prover
```

### Runtime Feature Flags

```toml
[features]
default = ["mock"]
mock = []           # Use mock runtime (no ZK, no testnet)
testnet = []        # Connect to testnet with real proofs
```

---

## Testing Strategy

### 1. Unit Tests (mock)
```bash
cargo test -p rfq-local-laws
```

### 2. Integration Tests (mock runtime)
```bash
USE_MOCK_RUNTIME=1 cargo test -p rfq-domain --test integration
```

### 3. Testnet Dry Run
```bash
# Submit but don't finalize
DRY_RUN=1 cargo run -p rfq-domain
```

### 4. Full Testnet
```bash
cargo run -p rfq-domain --features testnet
```

---

## Estimated Timeline

| Task | Effort | Dependencies |
|------|--------|--------------|
| Create ELF crate | 2h | SP1 toolchain |
| Build ELF binary | 1h | ELF crate |
| Wire Runtime | 4h | ELF binary |
| Async proof handling | 3h | Runtime wired |
| Error handling & retries | 2h | - |
| E2E testing on testnet | 4h | All above |

**Total: ~16 hours**

---

## Open Questions

1. **Proof generation location**: Self-hosted vs Succinct Network?
2. **Vault funding**: How do we fund maker/taker vaults for demo?
3. **Asset minting**: Do we need to mint demo tokens (dETH)?
4. **Fee handling**: Who pays for proof generation?

---

## References

- [Delta SDK Docs](https://docs.repyhlabs.dev/)
- [SP1 Book](https://succinctlabs.github.io/sp1/)
- [Local Laws Example](./crates/domain/examples/local_laws_demo.rs)
