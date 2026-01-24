# OTC RFQ Arena

A lightweight OTC RFQ (Request for Quote) market "arena" built on [Delta Network](https://docs.repyhlabs.dev/). Makers write quotes in plain English, the backend compiles them into machine-checkable guardrails ("Local Laws"), and Taker agents attempt to fill. Only fills satisfying the guardrails can settle.

## Overview

This demo showcases Delta's **Local Laws** - custom validation rules that are cryptographically enforced at settlement time. The trust problem in OTC trading is solved through guardrails + receipts, not through trusting counterparties.

### How It Works

1. **Maker posts a quote in English** (e.g., "Buy 10 dETH at most 2000 USDD, expires in 5 minutes")
2. **Backend compiles to guardrails** - max debit, expiry, allowed feeds, quorum rules, etc.
3. **Takers attempt to fill** - submitting price feed evidence
4. **Local Laws validate** - only compliant fills settle; attacks are rejected with clear reasons
5. **Receipts generated** - cryptographic proof of what happened and why

## Project Structure

```
crates/
├── models/          # Core data types (Quote, Constraints, Fill, Receipt)
├── local-laws/      # LocalLaws implementation for RFQ guardrails
├── compiler/        # LLM-based ESC compiler (English -> Guardrails)
├── feeds/           # Mock price feed servers
├── agents/          # Maker/Taker bot skeletons
└── domain/          # HTTP server with RFQ endpoints
    └── examples/    # Demo scripts
```

## Quick Start

### Prerequisites

- Rust 1.75+
- Access to Delta's private crate registry (configured in `.cargo/config.toml`)

### Run the Server

```bash
# Demo mode (mock compiler - no LLM needed)
USE_MOCK_COMPILER=1 API_PORT=3335 cargo run -p rfq-domain

# With LLM compiler
ANTHROPIC_API_KEY=sk-... API_PORT=3335 cargo run -p rfq-domain
```

### Run Tests

```bash
# All tests
cargo test

# Just local-laws tests
cargo test -p rfq-local-laws

# With output
cargo test -- --nocapture
```

### Run Examples

```bash
# LocalLaws validation demo
cargo run -p rfq-domain --example local_laws_demo
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/quotes` | List all active quotes |
| POST | `/quotes` | Create a new quote |
| GET | `/quotes/:id` | Get a specific quote |
| POST | `/quotes/:id/fill` | Attempt to fill a quote |
| GET | `/quotes/:id/receipts` | Get all fill receipts for a quote |

## Usage Examples

### Create a Quote

```bash
curl -X POST http://localhost:3335/quotes \
  -H "Content-Type: application/json" \
  -d '{
    "text": "I want to buy 10 dETH at most 2000 USDD each, expires in 5 minutes. Only accept prices from FeedA and FeedB, must be fresh within 5 seconds.",
    "maker_owner_id": "maker123",
    "maker_shard": 9
  }'
```

Response:
```json
{
  "quote": {
    "id": "f47c9b59-dad3-4472-888b-5614937710f5",
    "spec": {
      "asset": "dETH",
      "size": 10.0,
      "side": "buy",
      "limit_price": 2000.0,
      "currency": "USDD"
    },
    "constraints": {
      "max_debit": 20000000000000,
      "expiry_timestamp": 1769250653,
      "allowed_sources": ["FeedA", "FeedB"],
      "max_staleness_secs": 5,
      "quorum_count": 2,
      "require_atomic_dvp": true,
      "no_side_payments": true
    },
    "status": "active"
  }
}
```

### Fill a Quote

```bash
curl -X POST http://localhost:3335/quotes/QUOTE_ID/fill \
  -H "Content-Type: application/json" \
  -d '{
    "taker_owner_id": "taker456",
    "taker_shard": 9,
    "size": 10.0,
    "price": 1950.0,
    "feed_evidence": [
      {"source": "FeedA", "asset": "dETH", "price": 1950.0, "timestamp": 1769250388, "signature": "sig1"},
      {"source": "FeedB", "asset": "dETH", "price": 1952.0, "timestamp": 1769250388, "signature": "sig2"}
    ]
  }'
```

## Adversarial Scenarios

The system defends against various attacks:

| Attack | Guardrail | Result |
|--------|-----------|--------|
| **Stale Feed** - Using old price data | `max_staleness_secs` | `REJECTED: stale_feed` |
| **Spoofed Source** - Fake price feed | `allowed_sources` | `REJECTED: unauthorized_source` |
| **Insufficient Quorum** - Too few sources | `quorum_count` | `REJECTED: quorum_not_met` |
| **Price Manipulation** - Sources disagree | `quorum_tolerance_percent` | `REJECTED: quorum_not_met` |
| **Overfill** - Fill more than quote size | `max_fill_size` | `REJECTED: size_exceeds_max` |
| **Replay** - Fill already-filled quote | `nonce` | `REJECTED: already_filled` |
| **Unauthorized Taker** - Not in allowlist | `allowed_takers` | `REJECTED: unauthorized_taker` |
| **Side Payment** - Extra transfers | `no_side_payments` | `REJECTED: side_payment_detected` |

### Example: Stale Feed Attack

```bash
# Attacker submits 60-second-old feed data (max allowed: 5s)
curl -X POST http://localhost:3335/quotes/QUOTE_ID/fill \
  -H "Content-Type: application/json" \
  -d '{
    "taker_owner_id": "attacker",
    "taker_shard": 9,
    "size": 10.0,
    "price": 1800.0,
    "feed_evidence": [
      {"source": "FeedA", "asset": "dETH", "price": 1800.0, "timestamp": 1769250328, "signature": "sig"}
    ]
  }'
```

Response:
```json
{
  "result": {
    "status": "rejected",
    "reason": {
      "code": "stale_feed",
      "source": "FeedA",
      "feed_timestamp": 1769250328,
      "current_timestamp": 1769250388,
      "max_staleness_secs": 5
    }
  }
}
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `API_PORT` | `3000` | HTTP server port |
| `SHARD` | `9` | Delta shard ID |
| `KEYPAIR_PATH` | `keypair_9.json` | Path to Delta keypair |
| `RPC_URL` | `http://164.92.69.96:9000` | Delta testnet RPC |
| `USE_MOCK_COMPILER` | `false` | Use mock compiler (no LLM) |
| `LLM_PROVIDER` | `claude` | LLM provider (`claude` or `gpt`) |
| `ANTHROPIC_API_KEY` | - | Anthropic API key |
| `OPENAI_API_KEY` | - | OpenAI API key |

## How Local Laws Work

Local Laws are Delta's mechanism for custom validation logic that runs during settlement:

```rust
impl LocalLaws for RfqLocalLaws {
    type Input<'a> = RfqLocalLawsInput;  // Serializable guardrails

    fn validate<'a>(
        verifiables: &[VerifiableWithDiffs],
        ctx: &VerificationContext,
        input: &RfqLocalLawsInput,
    ) -> Result<(), LocalLawsError> {
        // Check expiry
        if ctx.timestamp > input.constraints.expiry_timestamp {
            return Err(LocalLawsError::Rejected("Quote expired"));
        }
        
        // Check feed freshness
        for evidence in &input.feed_evidence {
            if !evidence.is_fresh(input.constraints.max_staleness_secs, ctx.timestamp) {
                return Err(LocalLawsError::Rejected("Stale feed"));
            }
        }
        
        // ... more checks
        Ok(())
    }
}
```

When wired to the Delta Runtime, these validations are enforced cryptographically via ZK proofs.

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Maker (ESC)   │────>│  Domain Server   │<────│  Taker Agents   │
│  English Quote  │     │                  │     │  (Good + Evil)  │
└─────────────────┘     │  ┌────────────┐  │     └─────────────────┘
                        │  │  Compiler  │  │
                        │  │ (LLM/Mock) │  │     ┌─────────────────┐
                        │  └────────────┘  │<────│   Price Feeds   │
                        │  ┌────────────┐  │     │  (FeedA/FeedB)  │
                        │  │LocalLaws   │  │     └─────────────────┘
                        │  │ Validator  │  │
                        │  └────────────┘  │
                        └────────┬─────────┘
                                 │
                                 v
                        ┌──────────────────┐
                        │  Delta Testnet   │
                        │  (ZK Settlement) │
                        └──────────────────┘
```

## Development

### Adding New Guardrails

1. Add field to `QuoteConstraints` in `crates/models/src/constraints.rs`
2. Add validation logic in `crates/local-laws/src/lib.rs`
3. Add rejection reason in `crates/models/src/fill.rs`
4. Update mock compiler in `crates/domain/src/main.rs`

### Testing New Attack Scenarios

```bash
# Create a quote with specific constraints
curl -X POST http://localhost:3335/quotes -d '{"text": "...", ...}'

# Attempt malicious fill
curl -X POST http://localhost:3335/quotes/ID/fill -d '{"feed_evidence": [...]}'

# Check receipt
curl http://localhost:3335/quotes/ID/receipts
```

## License

MIT

## References

- [Delta Network Docs](https://docs.repyhlabs.dev/)
- [RFC.md](./RFC.md) - Original specification
