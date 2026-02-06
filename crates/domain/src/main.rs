//! RFQ Domain Server
//!
//! Main entry point for the OTC RFQ Arena domain.
//!
//! This server:
//! - Connects to Delta testnet via Runtime SDK
//! - Exposes HTTP endpoints for posting quotes and filling them
//! - Uses LLM to compile English quotes into guardrails
//! - Validates fills against local laws with ZK proofs
//!
//! ## Usage
//!
//! ```bash
//! # Mock mode (default) - no testnet connection
//! ANTHROPIC_API_KEY=... cargo run -- --mock --port 3335
//!
//! # Testnet mode - connects to Delta testnet
//! ANTHROPIC_API_KEY=... cargo run -- --config domain.yaml --port 3335
//! ```

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use delta_domain_sdk::base::crypto::ed25519::PrivKey;
use delta_domain_sdk::base::vaults::{Address, TokenKind, Vault, WritableNativeBalance};
use delta_domain_sdk::proving::mock;
use delta_domain_sdk::{execution::default_execute, Runtime, SdlState};
use delta_verifiable::types::debit_allowance::{AllowanceAmount, DebitAllowance, SignedDebitAllowance};
use delta_verifiable::types::VerifiableType;
use rfq_compiler::{Compiler, CompilerConfig};
use rfq_models::*;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod api_types;
mod config;
mod state;

use api_types::{ApiCreateQuoteResponse, ApiFillResponse, ApiQuote, ApiReceiptSummary};
use config::DomainConfig;
use state::DomainState;

/// CLI arguments
#[derive(Parser)]
#[command(name = "rfq-domain")]
#[command(about = "RFQ Arena - OTC quotes with ZK-proven local laws")]
struct CliArgs {
    /// Path to domain configuration file
    #[arg(short, long, default_value = "domain.yaml")]
    config: PathBuf,

    /// Port for the HTTP API server (overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Run in mock mode (no Delta testnet connection)
    #[arg(long)]
    mock: bool,
}

/// Type alias for our Runtime with mock proving
type DeltaRuntime = Runtime<mock::Client>;

/// Application state shared across handlers
pub struct AppState {
    /// Domain state (quotes, receipts)
    pub domain: Arc<DomainState>,
    /// Delta Runtime (for SDL submission and proving)
    pub runtime: Arc<RwLock<DeltaRuntime>>,
    /// Domain operator keypair (for signing transfers)
    pub keypair: Arc<PrivKey>,
    /// LLM compiler for quotes
    pub compiler: Compiler,
    /// Configuration
    pub config: DomainConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rfq_domain=debug".into()),
        )
        .init();

    tracing::info!("Starting RFQ Domain Server...");

    // Parse CLI args
    let args = CliArgs::parse();

    // Load configuration
    let mut config = if args.config.exists() {
        DomainConfig::load_from(&args.config)
            .with_context(|| format!("Failed to load config from {:?}", args.config))?
    } else {
        tracing::warn!("Config file {:?} not found, using defaults", args.config);
        DomainConfig::default()
    };

    // Apply CLI overrides
    if let Some(port) = args.port {
        config.api_port = port;
    }
    if args.mock {
        config.mock_mode = true;
    }

    tracing::info!(
        "Configuration: shard={}, port={}, mock_mode={}",
        config.shard,
        config.api_port,
        config.mock_mode
    );

    // Initialize LLM compiler
    if config.llm_api_key.is_empty() {
        tracing::error!("No LLM API key configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY");
        std::process::exit(1);
    }

    tracing::info!("Using LLM compiler ({} provider)", config.llm_provider);
    let compiler = Compiler::new(CompilerConfig {
        llm: config.llm_provider.clone(),
        api_key: config.llm_api_key.clone(),
        model: if config.llm_provider == "claude" {
            "claude-sonnet-4-20250514".to_string()
        } else {
            "gpt-4o-mini".to_string()
        },
    });

    // Initialize Delta Runtime
    let (runtime, keypair) = init_runtime(&config).await?;
    tracing::info!("Delta Runtime initialized (mock_mode={})", config.mock_mode);

    // Create application state
    let state = Arc::new(AppState {
        domain: DomainState::new(),
        runtime: Arc::new(RwLock::new(runtime)),
        keypair: Arc::new(keypair),
        compiler,
        config: config.clone(),
    });

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // Quote endpoints
        .route("/quotes", get(list_quotes))
        .route("/quotes", post(create_quote))
        .route("/quotes/:id", get(get_quote))
        .route("/quotes/:id/fill", post(fill_quote))
        // Receipt endpoints
        .route("/quotes/:id/receipts", get(get_receipts))
        // CORS
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.api_port);
    tracing::info!("HTTP server listening on {}", addr);
    tracing::info!("Endpoints:");
    tracing::info!("  GET  /health              - Health check");
    tracing::info!("  GET  /quotes              - List quotes");
    tracing::info!("  POST /quotes              - Create quote from text");
    tracing::info!("  GET  /quotes/:id         - Get quote");
    tracing::info!("  POST /quotes/:id/fill    - Fill quote");
    tracing::info!("  GET  /quotes/:id/receipts - Get receipts");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Initialize the Delta Runtime
/// Returns the runtime and the domain keypair
async fn init_runtime(config: &DomainConfig) -> Result<(DeltaRuntime, PrivKey)> {
    let shard = NonZero::new(config.shard).context("Invalid shard (cannot be 0)")?;

    // Load or generate keypair
    let keypair = if std::path::Path::new(&config.keypair_path).exists() {
        let key_str = std::fs::read_to_string(&config.keypair_path)?;
        let key_str = key_str.trim().trim_matches('"');
        let key_bytes = bs58::decode(key_str)
            .into_vec()
            .context("Failed to decode base58 keypair")?;
        let key_array: [u8; 32] = key_bytes
            .try_into()
            .map_err(|v: Vec<u8>| anyhow::anyhow!("Expected 32 bytes, got {}", v.len()))?;
        PrivKey::from_bytes(&key_array)
    } else {
        tracing::warn!("Keypair file not found, generating new keypair");
        PrivKey::generate()
    };

    tracing::info!("Using keypair: {}", keypair.pub_key().owner());

    // Create mock proving client with our local laws
    let proving_client = mock::Client::global_laws()
        .with_local_laws::<rfq_local_laws::RfqLocalLaws>();

    // Clone keypair for return value (before it's moved into builder)
    let keypair_clone = keypair.clone();

    // Build runtime
    let runtime = if config.mock_mode {
        // Mock mode: use mock RPC with pre-populated vaults
        let owner = keypair.pub_key().owner();
        let vault_address = Address::new(owner, shard.get());
        let mut vault = Vault::new(shard);
        vault.set_balance(1_000_000_000_000_000); // 1 Quadrillion Plancks for testing (1M USDD equivalent)

        tracing::info!("Mock mode: Pre-populated vault {} with 1Q Plancks", vault_address);

        Runtime::builder(shard, keypair)
            .with_mock_rpc(HashMap::from([(vault_address, vault)]))
            .with_proving_client(proving_client)
            .build()
            .await
            .context("Failed to build mock runtime")?
    } else {
        // Testnet mode: connect to real RPC
        tracing::info!("Connecting to Delta testnet at {}", config.rpc_url);

        Runtime::builder(shard, keypair)
            .with_rpc(&config.rpc_url)
            .with_proving_client(proving_client)
            .build()
            .await
            .context("Failed to build testnet runtime")?
    };

    // Run the runtime in background
    let runtime_clone = runtime.clone();
    tokio::spawn(async move {
        if let Err(e) = runtime_clone.run().await {
            tracing::error!("Runtime error: {}", e);
        }
    });

    Ok((runtime, keypair_clone))
}

// =============================================================================
// Handlers
// =============================================================================

/// Health check endpoint
async fn health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "shard": state.config.shard,
        "mock_mode": state.config.mock_mode,
    }))
}

/// List all quotes (including filled and expired)
async fn list_quotes(State(state): State<Arc<AppState>>) -> Json<Vec<ApiQuote>> {
    let mut quotes = state.domain.get_all_quotes().await;
    
    // Update status for expired quotes
    for quote in &mut quotes {
        if quote.status == QuoteStatus::Active && quote.is_expired() {
            quote.status = QuoteStatus::Expired;
            // Persist the updated status
            state.domain.update_quote(quote.clone()).await;
        }
    }
    
    let api_quotes: Vec<ApiQuote> = quotes.iter().map(ApiQuote::from).collect();
    Json(api_quotes)
}

/// Get a specific quote
async fn get_quote(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiQuote>, StatusCode> {
    state
        .domain
        .get_quote(&id)
        .await
        .map(|q| Json(ApiQuote::from(&q)))
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new quote from English text
async fn create_quote(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateQuoteRequest>,
) -> Result<Json<ApiCreateQuoteResponse>, (StatusCode, String)> {
    tracing::info!("Creating quote from text: {}", request.text);

    // Generate quote ID
    let quote_id = Uuid::new_v4();
    let quote_id_bytes: [u8; 32] = {
        let mut bytes = [0u8; 32];
        bytes[..16].copy_from_slice(quote_id.as_bytes());
        bytes
    };

    // Get next nonce (simplified - in production, get from vault)
    let nonce = 1u64;

    // Compile the quote using LLM
    let (spec, constraints) = state
        .compiler
        .compile(&request.text, quote_id_bytes, nonce)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to compile quote: {}", e)))?;

    // Create the quote
    let now = chrono::Utc::now();
    let quote = Quote {
        id: quote_id,
        spec: spec.clone(),
        constraints: constraints.clone(),
        status: QuoteStatus::Active,
        created_at: now,
        expires_at: constraints.expiry_datetime(),
        maker_owner_id: request.maker_owner_id.clone(),
        maker_vault_address: format!("{},{}", request.maker_owner_id, request.maker_shard),
        original_text: request.text.clone(),
    };

    // Store the quote
    state.domain.add_quote(quote.clone()).await;
    tracing::info!("Quote created: {}", quote_id);

    // Generate summary
    let summary = rfq_compiler::summarize_constraints(&constraints);

    // Return flattened API response
    Ok(Json(ApiCreateQuoteResponse {
        quote: ApiQuote::from(&quote),
        constraints_summary: summary,
        message: "Quote created successfully. The Local Law has been compiled and will enforce your constraints cryptographically.".to_string(),
    }))
}

/// Attempt to fill a quote
async fn fill_quote(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<FillRequest>,
) -> Result<Json<ApiFillResponse>, (StatusCode, String)> {
    tracing::info!(
        "Fill attempt for quote {}: taker={}",
        id,
        request.taker_owner_id
    );

    // Get the quote
    let mut quote = state
        .domain
        .get_quote(&id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Quote not found".to_string()))?;

    // Check if quote is still active
    if !quote.is_active() {
        let reason = if quote.is_expired() {
            RejectionReason::QuoteExpired {
                expired_at: quote.expires_at,
                attempted_at: chrono::Utc::now(),
            }
        } else {
            RejectionReason::AlreadyFilled {
                filled_at: chrono::Utc::now(),
            }
        };

        let fill_attempt = FillAttempt {
            id: Uuid::new_v4(),
            quote_id: id,
            taker_owner_id: request.taker_owner_id,
            taker_shard: request.taker_shard,
            size: request.size,
            price: request.price,
            feed_evidence: request.feed_evidence,
            attempted_at: chrono::Utc::now(),
        };

        let receipt = FillReceipt::new(
            quote.clone(),
            quote.constraints.clone(),
            fill_attempt.clone(),
            FillResult::Rejected {
                fill_id: fill_attempt.id,
                reason,
            },
        );

        state.domain.add_receipt(id, receipt.clone()).await;
        return Ok(Json(ApiFillResponse::from(&receipt)));
    }

    // Create fill attempt
    let fill_attempt = FillAttempt {
        id: Uuid::new_v4(),
        quote_id: id,
        taker_owner_id: request.taker_owner_id.clone(),
        taker_shard: request.taker_shard,
        size: request.size,
        price: request.price,
        feed_evidence: request.feed_evidence.clone(),
        attempted_at: chrono::Utc::now(),
    };

    // Validate against local laws
    let current_timestamp = chrono::Utc::now().timestamp() as u64;
    let local_laws_input = rfq_local_laws::RfqLocalLawsInput {
        constraints: quote.constraints.clone(),
        taker_owner_id: request.taker_owner_id.clone(),
        fill_size: (request.size * 1_000_000_000.0) as u64,
        fill_price: (request.price * request.size * 1_000_000_000.0) as u64,
        feed_evidence: request.feed_evidence.clone(),
        current_timestamp,
        transfer_leg_count: 2, // Assuming atomic DvP
        has_extra_transfers: false,
    };

    let result = rfq_local_laws::validate_fill(&local_laws_input);

    let fill_result = match result {
        Ok(()) => {
            // Fill accepted! Submit to Delta for proof
            quote.status = QuoteStatus::Filled;
            state.domain.update_quote(quote.clone()).await;

            // Create fill context for transfer verifiables
            let fill_ctx = FillContext {
                maker_owner_id: quote.maker_owner_id.clone(),
                taker_owner_id: request.taker_owner_id.clone(),
                maker_pays: local_laws_input.fill_price,
                taker_pays: local_laws_input.fill_size,
            };

            // Submit SDL to Delta Runtime with actual transfers
            let sdl_hash = submit_fill_to_delta(&state, &local_laws_input, &fill_ctx).await;

            FillResult::Accepted {
                fill_id: fill_attempt.id,
                sdl_hash,
                settlement: SettlementDetails {
                    maker_debit: local_laws_input.fill_price,
                    maker_credit: local_laws_input.fill_size,
                    taker_debit: local_laws_input.fill_size,
                    taker_credit: local_laws_input.fill_price,
                    asset: quote.spec.asset.clone(),
                    currency: quote.spec.currency.clone(),
                    settled_at: chrono::Utc::now(),
                },
            }
        }
        Err(reason) => FillResult::Rejected {
            fill_id: fill_attempt.id,
            reason,
        },
    };

    let receipt = FillReceipt::new(
        quote.clone(),
        quote.constraints.clone(),
        fill_attempt,
        fill_result,
    );

    state.domain.add_receipt(id, receipt.clone()).await;

    tracing::info!(
        "Fill result for quote {}: {}",
        id,
        if receipt.is_accepted() {
            "ACCEPTED"
        } else {
            "REJECTED"
        }
    );

    Ok(Json(ApiFillResponse::from(&receipt)))
}

/// Context for submitting a fill to Delta
struct FillContext {
    /// Maker's owner ID (base58 or arbitrary string)
    maker_owner_id: String,
    /// Taker's owner ID (base58 or arbitrary string)
    taker_owner_id: String,
    /// Amount maker pays (in plancks) - the price * size
    maker_pays: u64,
    /// Amount taker pays (in plancks) - the asset size
    taker_pays: u64,
}

/// Convert an owner ID string to an OwnerId
/// 
/// Tries to parse as base58 first. If that fails, derives a deterministic
/// OwnerId by hashing the string (useful for demo/mock mode).
fn parse_or_derive_owner_id(id_str: &str) -> delta_domain_sdk::base::crypto::OwnerId {
    // Try base58 decode first
    if let Ok(bytes) = bs58::decode(id_str).into_vec() {
        if bytes.len() == 32 {
            let arr: [u8; 32] = bytes.try_into().unwrap();
            return delta_domain_sdk::base::crypto::OwnerId::from(arr);
        }
    }
    
    // Fallback: derive deterministic OwnerId from string via SHA256
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(id_str.as_bytes());
    let hash: [u8; 32] = hasher.finalize().into();
    
    tracing::debug!("Derived owner ID for '{}': {}", id_str, bs58::encode(&hash).into_string());
    delta_domain_sdk::base::crypto::OwnerId::from(hash)
}

/// Submit a fill to Delta Runtime for SDL creation and proof
///
/// This creates the actual transfer verifiables:
/// 1. Maker sends currency to taker (fill_price)
/// 2. Taker sends asset to maker (fill_size)
async fn submit_fill_to_delta(
    state: &AppState,
    local_laws_input: &rfq_local_laws::RfqLocalLawsInput,
    fill_ctx: &FillContext,
) -> String {
    use delta_serializers::bytes::BytesSerializer;
    use delta_serializers::serializer::Serializer;

    let runtime: tokio::sync::RwLockReadGuard<'_, DeltaRuntime> = state.runtime.read().await;
    let shard = state.config.shard;

    // Parse or derive owner IDs
    let maker_owner = parse_or_derive_owner_id(&fill_ctx.maker_owner_id);
    let taker_owner = parse_or_derive_owner_id(&fill_ctx.taker_owner_id);
    let domain_owner = state.keypair.pub_key().owner();

    let maker_address = Address::new(maker_owner, shard);
    let taker_address = Address::new(taker_owner, shard);
    let domain_address = Address::new(domain_owner, shard);

    // Get the next nonce for domain vault (both transfers debit from domain)
    let base_nonce = match runtime.domain_view().next_nonce(&domain_owner) {
        Ok(nonce) => nonce,
        Err(e) => {
            tracing::error!("Failed to get domain nonce: {}", e);
            return format!("error_nonce_{}", uuid::Uuid::new_v4());
        }
    };

    tracing::info!(
        "Creating transfer verifiables: maker={} taker={} domain={} base_nonce={}",
        maker_address, taker_address, domain_address, base_nonce
    );

    // Create the transfer verifiables for atomic DvP (Delivery vs Payment)
    // For simplicity in this demo, the domain acts as intermediary:
    // - Domain credits taker with maker's payment (currency)
    // - Domain credits maker with taker's asset (simulated as native token)
    //
    // In a real implementation, you'd have proper asset tokens and direct transfers.

    // Transfer 1: Domain -> Taker (the currency/payment from maker)
    // Uses base_nonce for the first transfer
    let domain_to_taker = DebitAllowance {
        credited: taker_address,
        allowances: BTreeMap::from([(
            TokenKind::Native,
            AllowanceAmount::Fungible(fill_ctx.maker_pays),
        )]),
        new_nonce: base_nonce,
        debited_shard: shard,
    };

    let v1 = match SignedDebitAllowance::sign(domain_to_taker, state.keypair.as_ref()) {
        Ok(signed) => VerifiableType::DebitAllowance(signed),
        Err(e) => {
            tracing::error!("Failed to sign domain->taker transfer: {}", e);
            return format!("error_sign_{}", uuid::Uuid::new_v4());
        }
    };

    // Transfer 2: Domain -> Maker (the asset from taker, simulated as native token)
    // Uses base_nonce + 1 for the second transfer
    let domain_to_maker = DebitAllowance {
        credited: maker_address,
        allowances: BTreeMap::from([(
            TokenKind::Native,
            AllowanceAmount::Fungible(fill_ctx.taker_pays),
        )]),
        new_nonce: base_nonce + 1,
        debited_shard: shard,
    };

    let v2 = match SignedDebitAllowance::sign(domain_to_maker, state.keypair.as_ref()) {
        Ok(signed) => VerifiableType::DebitAllowance(signed),
        Err(e) => {
            tracing::error!("Failed to sign domain->maker transfer: {}", e);
            return format!("error_sign_{}", uuid::Uuid::new_v4());
        }
    };

    let verifiables = vec![v1, v2];
    tracing::info!("Created {} verifiables for fill", verifiables.len());

    // Apply verifiables (creates state diffs)
    if let Err(e) = runtime.apply(default_execute(verifiables)).await {
        tracing::error!("Failed to apply verifiables: {}", e);
        return format!("error_apply_{}", uuid::Uuid::new_v4());
    }

    // Submit to get SDL hash
    let sdl_hash = match runtime.submit().await {
        Ok(Some(hash)) => hash,
        Ok(None) => {
            tracing::info!("No state changes to submit");
            return format!("no_changes_{}", uuid::Uuid::new_v4());
        }
        Err(e) => {
            tracing::error!("Failed to submit SDL: {}", e);
            return format!("error_submit_{}", uuid::Uuid::new_v4());
        }
    };

    tracing::info!("SDL submitted: {:?}", sdl_hash);

    // Serialize local laws input for proof
    let input_bytes = match BytesSerializer::serialize(local_laws_input) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to serialize local laws input: {}", e);
            return format!("{:?}", sdl_hash);
        }
    };

    // Subscribe to updates BEFORE starting prove (to not miss the Proven event)
    let mut updates = runtime.updates();

    // Start proof generation (async task)
    if let Err(e) = runtime.prove_with_local_laws_input(sdl_hash, input_bytes).await {
        tracing::error!("Failed to start proving: {}", e);
        return format!("{:?}", sdl_hash);
    }

    tracing::info!("Proving started for SDL: {:?}", sdl_hash);

    // Wait for proof to complete (SdlState::Proven)
    let proof_timeout = tokio::time::Duration::from_secs(60);
    let proven = tokio::time::timeout(proof_timeout, async {
        loop {
            match updates.recv().await {
                Ok(update) => {
                    if update.sdl_hash == sdl_hash {
                        tracing::debug!("SDL update: {:?} -> {:?}", sdl_hash, update.new_state);
                        match update.new_state {
                            SdlState::Proven => {
                                return Ok(());
                            }
                            SdlState::ProvingFailed(err) => {
                                return Err(format!("Proving failed: {}", err));
                            }
                            _ => continue,
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Updates channel error: {:?}", e);
                    // Channel lagged, try again
                    continue;
                }
            }
        }
    })
    .await;

    match proven {
        Ok(Ok(())) => {
            tracing::info!("Proof generated for SDL: {:?}", sdl_hash);
        }
        Ok(Err(e)) => {
            tracing::error!("Proving failed: {}", e);
            return format!("{:?}", sdl_hash);
        }
        Err(_) => {
            tracing::error!("Proof generation timed out for SDL: {:?}", sdl_hash);
            return format!("{:?}", sdl_hash);
        }
    }

    // NOW submit proof to base layer (proof is stored)
    if let Err(e) = runtime.submit_proof(sdl_hash).await {
        tracing::error!("Failed to submit proof: {}", e);
        return format!("{:?}", sdl_hash);
    }

    tracing::info!("Proof submitted for SDL: {:?}", sdl_hash);
    format!("{:?}", sdl_hash)
}

/// Get receipts for a quote
async fn get_receipts(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Json<Vec<ApiReceiptSummary>> {
    let receipts = state.domain.get_receipts(&id).await;
    let api_receipts: Vec<ApiReceiptSummary> = receipts.iter().map(ApiReceiptSummary::from).collect();
    Json(api_receipts)
}
