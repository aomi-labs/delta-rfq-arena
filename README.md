# OTC RFQ Arena

A lightweight OTC RFQ (Request for Quote) market "arena" built on [Delta Network](https://docs.delta.network/). Makers write quotes in plain English, the backend compiles them into machine-checkable guardrails ("Local Laws"), and Taker agents attempt to fill. Only fills satisfying the guardrails can settle - enforced with ZK proofs.

## Overview

This demo showcases Delta's **Local Laws** - custom validation rules that are cryptographically enforced at settlement time. The trust problem in OTC trading is solved through guardrails + receipts, not through trusting counterparties.

### How It Works

1. **Maker posts a quote in English** (e.g., "Buy 10 dETH at most 2000 USDD, expires in 5 minutes")
2. **Backend compiles to guardrails** - LLM extracts max debit, expiry, allowed feeds, quorum rules, etc.
3. **Takers attempt to fill** - submitting price feed evidence
4. **Local Laws validate** - only compliant fills settle; attacks are rejected with clear reasons
5. **Delta Runtime proves** - ZK proof generated and submitted to testnet
6. **Receipts generated** - cryptographic proof of what happened and why

## Project Structure

```
delta-rfq-arena/
├── start.sh              # Start all services (FE + BE)
├── crates/
│   ├── models/           # Core data types (Quote, Constraints, Fill, Receipt)
│   ├── local-laws/       # LocalLaws implementation for RFQ guardrails
│   ├── local-laws-elf/   # SP1 zkVM program for local laws proofs
│   ├── compiler/         # LLM-based compiler (English -> Guardrails)
│   ├── feeds/            # Mock price feed servers
│   └── domain/           # HTTP server + Delta Runtime integration
│       ├── src/
│       │   ├── main.rs   # Server entry point
│       │   ├── config.rs # YAML config loading
│       │   └── state.rs  # In-memory quote/receipt storage
│       ├── domain.yaml   # Testnet configuration
│       └── keypair_9.json # Pre-funded test keypair (shard 9)
└── web/                  # Next.js frontend with Aomi agent integration
```

## Quick Start

### Prerequisites

- Rust 1.75+
- Node.js 18+ (for frontend)
- Access to Delta's private crate registry (configured in `.cargo/config.toml`)
- Anthropic API key (for LLM quote compilation)

### One-Command Start

```bash
# Set your API key
export ANTHROPIC_API_KEY=your_key_here

# Start everything (frontend + backend)
./start.sh

# Or with custom ports
./start.sh --rfq-port 3335 --aomi-port 8080

# Mock mode (no Delta testnet connection)
./start.sh --mock
```

This starts:
- **RFQ Domain Server** on port 3335 (quotes, fills, local laws)
- **Aomi Agent Server** on port 8080 (AI agent backend, if installed)
- **Frontend** on port 3000 (Next.js web UI)

### Manual Start (Alternative)

```bash
# Terminal 1: Backend
export ANTHROPIC_API_KEY=your_key
cd crates/domain
cargo run -- --mock --port 3335

# Terminal 2: Frontend
cd web
npm install
NEXT_PUBLIC_API_URL=http://localhost:3335 npm run dev
```

### Start Script Options

```
./start.sh [OPTIONS]

Options:
  --rfq-port PORT    Port for RFQ Domain server (default: 3335)
  --aomi-port PORT   Port for Aomi Agent server (default: 8080)
  --fe-port PORT     Port for Frontend dev server (default: 3000)
  --no-fe            Skip starting the frontend
  --no-aomi          Skip starting the Aomi agent
  --mock             Run RFQ server in mock mode (no Delta testnet)
  -h, --help         Show help message
```

### Run Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p rfq-models
cargo test -p rfq-local-laws
cargo test -p rfq-compiler
cargo test -p rfq-domain

# With output
cargo test -- --nocapture
```

### Run Examples

```bash
# LocalLaws validation demo
cargo run -p rfq-domain --example local_laws_demo
```

### Build ZK Programs

```bash
# Build local laws ELF (for fill validation proofs)
cd crates/local-laws-elf && cargo prove build
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

### Domain Server Config (`crates/domain/domain.yaml`)

```yaml
shard: 9
rpc_url: "http://164.92.69.96:9000"
keypair_path: "keypair_9.json"
api_port: 3335
mock_mode: false
llm_provider: "claude"  # or "gpt"
```

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Yes* | For Claude LLM quote compilation |
| `OPENAI_API_KEY` | Yes* | For GPT LLM quote compilation |

*One of these is required depending on `llm_provider` setting.

### Frontend Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `NEXT_PUBLIC_API_URL` | `http://localhost:8099` | RFQ Domain server URL |
| `NEXT_PUBLIC_BACKEND_URL` | `http://localhost:8080` | Aomi Agent server URL |

### CLI Arguments

```bash
cargo run -p rfq-domain -- [OPTIONS]

Options:
  -c, --config <PATH>   Config file path (default: domain.yaml)
  -p, --port <PORT>     Override API port
  --mock                Run in mock mode (no testnet connection)
```

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

## Command Reference

### Quick Copy-Paste

```bash
# One command - start everything
export ANTHROPIC_API_KEY=your_key
./start.sh --mock

# Or manually:
# Terminal 1: Backend
cd crates/domain && cargo run -- --mock --port 3335

# Terminal 2: Frontend  
cd web && NEXT_PUBLIC_API_URL=http://localhost:3335 npm run dev

# Terminal 3: Test API
curl -X POST http://localhost:3335/quotes \
  -H "Content-Type: application/json" \
  -d '{"text":"Buy 1 dETH max 2000 USDD, 5 min","maker_owner_id":"me","maker_shard":9}'
```

### All Commands

| Command | Description |
|---------|-------------|
| `./start.sh` | Start all services (FE + BE) |
| `./start.sh --mock` | Start in mock mode (no testnet) |
| `./start.sh --rfq-port 3335 --aomi-port 8080` | Custom ports |
| `cargo run -p rfq-domain -- --mock` | Run backend only (mock) |
| `cargo run -p rfq-domain --example local_laws_demo` | Run local laws demo |
| `cargo test --workspace` | Run all tests |
| `cargo test -p rfq-local-laws` | Test local laws |
| `cargo check --workspace` | Check compilation |
| `cd crates/local-laws-elf && cargo prove build` | Build ZK ELF |
| `cd web && npm run dev` | Run frontend dev server |
| `cd web && npm run build` | Build frontend for production |

## Development

### Adding New Guardrails

1. Add field to `QuoteConstraints` in `crates/models/src/constraints.rs`
2. Add validation logic in `crates/local-laws/src/lib.rs`
3. Add rejection reason in `crates/models/src/fill.rs`
4. Update LLM prompt in `crates/compiler/src/lib.rs`
5. Rebuild ELF: `cd crates/local-laws-elf && cargo prove build`

### Testing New Attack Scenarios

```bash
# Create a quote with specific constraints
curl -X POST http://localhost:3335/quotes -d '{"text": "...", ...}'

# Attempt malicious fill
curl -X POST http://localhost:3335/quotes/ID/fill -d '{"feed_evidence": [...]}'

# Check receipt
curl http://localhost:3335/quotes/ID/receipts
```

## Delta SDK Integration

The domain server integrates with Delta's Runtime SDK for ZK-proven settlement:

```rust
// Build runtime with mock proving client + local laws
let runtime = Runtime::builder(shard, keypair)
    .with_mock_rpc(HashMap::from([(vault_address, vault)]))
    .with_proving_client(mock::Client::global_laws().with_local_laws::<RfqLocalLaws>())
    .build()
    .await?;

// SDL submission flow
runtime.apply(default_execute(verifiables)).await?;
let sdl_hash = runtime.submit().await?;
runtime.prove_with_local_laws_input(sdl_hash, input_bytes).await?;
runtime.submit_proof(sdl_hash).await?;
```

### Testnet Credentials (Pre-configured)

| Setting | Value |
|---------|-------|
| Shard | 9 |
| RPC | `http://164.92.69.96:9000` |
| Keypair | `keypair_9.json` (pre-funded) |

## License

MIT

## References

- [Delta Network Docs](https://docs.delta.network/)
- [Delta SDK Mocks](https://docs.delta.network/docs/build/mocks)
- [RFC.md](./RFC.md) - Original specification
