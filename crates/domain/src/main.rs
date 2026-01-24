//! RFQ Domain Server
//!
//! Main entry point for the OTC RFQ Arena domain.
//! 
//! This server:
//! - Connects to delta testnet via the Runtime (when testnet feature enabled)
//! - Exposes HTTP endpoints for posting quotes and filling them
//! - Uses LLM to compile English quotes into guardrails
//! - Validates fills against local laws
//!
//! ## Features
//!
//! - `mock` (default): Use mock runtime, no testnet connection
//! - `testnet`: Connect to delta testnet with real ZK proofs

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rfq_compiler::{Compiler, CompilerConfig};
use rfq_models::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod config;
mod runtime;
mod state;

use config::DomainConfig;
use runtime::RuntimeHandle;
use state::DomainState;

/// Application state shared across handlers
pub struct AppState {
    /// Domain state (quotes, receipts)
    pub domain: Arc<DomainState>,
    /// Delta runtime (optional - may not be connected)
    pub runtime: Option<Arc<RwLock<RuntimeHandle>>>,
    /// LLM compiler for quotes
    pub compiler: Option<Compiler>,
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

    // Load configuration
    let config = DomainConfig::default();
    tracing::info!("Configuration loaded: shard={}, api_port={}", config.shard, config.api_port);

    // Initialize compiler (if API key is set and not using mock mode)
    let compiler = if !config.use_mock_compiler && !config.llm_api_key.is_empty() {
        tracing::info!("Using LLM compiler ({} provider)", config.llm_provider);
        Some(Compiler::new(CompilerConfig {
            llm: config.llm_provider.clone(),
            api_key: config.llm_api_key.clone(),
            model: if config.llm_provider == "claude" {
                "claude-3-5-sonnet-20241022".to_string()
            } else {
                "gpt-4".to_string()
            },
        }))
    } else {
        tracing::info!("Using mock compiler for demo mode");
        None
    };

    // Initialize delta runtime
    let runtime = match runtime::init_runtime(&config).await {
        Ok(rt) => {
            tracing::info!("Runtime initialized successfully");
            Some(rt)
        }
        Err(e) => {
            tracing::warn!("Failed to initialize runtime: {}. Running in offline mode.", e);
            None
        }
    };

    // Create application state
    let state = Arc::new(AppState {
        domain: DomainState::new(),
        runtime,
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
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.api_port);
    tracing::info!("HTTP server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// =============================================================================
// Handlers
// =============================================================================

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// List all active quotes
async fn list_quotes(State(state): State<Arc<AppState>>) -> Json<Vec<Quote>> {
    let quotes = state.domain.get_active_quotes().await;
    Json(quotes)
}

/// Get a specific quote
async fn get_quote(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Quote>, StatusCode> {
    state
        .domain
        .get_quote(&id)
        .await
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Create a new quote from English text
async fn create_quote(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateQuoteRequest>,
) -> Result<Json<CreateQuoteResponse>, (StatusCode, String)> {
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

    // Compile the quote
    let (spec, constraints) = if let Some(compiler) = &state.compiler {
        // Use LLM to compile
        compiler
            .compile(&request.text, quote_id_bytes, nonce)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to compile quote: {}", e)))?
    } else {
        // Mock compilation for demo
        mock_compile(&request.text, quote_id_bytes, nonce)
            .map_err(|e| (StatusCode::BAD_REQUEST, e))?
    };

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

    Ok(Json(CreateQuoteResponse {
        quote,
        constraints_summary: summary,
    }))
}

/// Attempt to fill a quote
async fn fill_quote(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<FillRequest>,
) -> Result<Json<FillReceipt>, (StatusCode, String)> {
    tracing::info!("Fill attempt for quote {}: taker={}", id, request.taker_owner_id);

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
        return Ok(Json(receipt));
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
    let input = rfq_local_laws::RfqLocalLawsInput {
        constraints: quote.constraints.clone(),
        taker_owner_id: request.taker_owner_id.clone(),
        fill_size: (request.size * 1_000_000_000.0) as u64,
        fill_price: (request.price * request.size * 1_000_000_000.0) as u64,
        feed_evidence: request.feed_evidence.clone(),
        current_timestamp,
        transfer_leg_count: 2, // Assuming atomic DvP
        has_extra_transfers: false,
    };

    let result = rfq_local_laws::validate_fill(&input);

    let fill_result = match result {
        Ok(()) => {
            // Fill accepted!
            quote.status = QuoteStatus::Filled;
            state.domain.update_quote(quote.clone()).await;

            FillResult::Accepted {
                fill_id: fill_attempt.id,
                sdl_hash: format!("mock_sdl_{}", fill_attempt.id),
                settlement: SettlementDetails {
                    maker_debit: input.fill_price,
                    maker_credit: input.fill_size,
                    taker_debit: input.fill_size,
                    taker_credit: input.fill_price,
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
        if receipt.is_accepted() { "ACCEPTED" } else { "REJECTED" }
    );

    Ok(Json(receipt))
}

/// Get receipts for a quote
async fn get_receipts(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Json<Vec<FillReceipt>> {
    let receipts = state.domain.get_receipts(&id).await;
    Json(receipts)
}

// =============================================================================
// Mock Compilation (for demo without LLM)
// =============================================================================

fn mock_compile(
    text: &str,
    quote_id: [u8; 32],
    nonce: u64,
) -> Result<(QuoteSpec, QuoteConstraints), String> {
    // Parse simple patterns from the text
    let text_lower = text.to_lowercase();

    // Detect buy/sell
    let side = if text_lower.contains("buy") {
        Side::Buy
    } else if text_lower.contains("sell") {
        Side::Sell
    } else {
        return Err("Could not determine buy/sell side".to_string());
    };

    // Extract size (look for numbers before asset names)
    let size = extract_number(&text_lower).unwrap_or(1.0);

    // Detect asset
    let asset = if text_lower.contains("deth") || text_lower.contains("eth") {
        "dETH"
    } else {
        "dETH" // Default
    };

    // Detect max price
    let max_price = if let Some(idx) = text_lower.find("at most") {
        extract_number(&text_lower[idx..])
    } else if let Some(idx) = text_lower.find("max") {
        extract_number(&text_lower[idx..])
    } else {
        Some(2000.0) // Default
    };

    // Detect expiry (in minutes)
    let expiry_minutes = if let Some(idx) = text_lower.find("expire") {
        extract_number(&text_lower[idx..]).map(|n| n as u64).unwrap_or(5)
    } else {
        5 // Default 5 minutes
    };

    // Detect allowed sources
    let mut allowed_sources = vec![];
    if text_lower.contains("feeda") {
        allowed_sources.push("FeedA".to_string());
    }
    if text_lower.contains("feedb") {
        allowed_sources.push("FeedB".to_string());
    }
    if allowed_sources.is_empty() {
        allowed_sources = vec!["FeedA".to_string(), "FeedB".to_string()];
    }

    // Detect staleness
    let max_staleness = if let Some(idx) = text_lower.find("fresh") {
        extract_number(&text_lower[idx..]).map(|n| n as u64).unwrap_or(5)
    } else {
        5 // Default 5 seconds
    };

    let spec = QuoteSpec {
        asset: asset.to_string(),
        size,
        side,
        limit_price: max_price,
        currency: "USDD".to_string(),
    };

    let now = chrono::Utc::now().timestamp() as u64;
    let constraints = QuoteConstraints {
        quote_id,
        max_debit: max_price.map(|p| (p * size * 1_000_000_000.0) as u64).unwrap_or(u64::MAX),
        min_credit: None,
        expiry_timestamp: now + (expiry_minutes * 60),
        allowed_sources,
        max_staleness_secs: max_staleness,
        quorum_count: 2,
        quorum_tolerance_percent: 0.5,
        allowed_takers: vec![],
        allowed_assets: vec![asset.to_string()],
        require_atomic_dvp: text_lower.contains("atomic") || !text_lower.contains("side"),
        no_side_payments: text_lower.contains("no side") || !text_lower.contains("side-payment"),
        nonce,
        max_fill_size: (size * 1_000_000_000.0) as u64,
    };

    Ok((spec, constraints))
}

fn extract_number(text: &str) -> Option<f64> {
    let mut num_str = String::new();
    let mut found_digit = false;

    for c in text.chars() {
        if c.is_ascii_digit() || c == '.' || c == ',' {
            if c != ',' {
                num_str.push(c);
            }
            found_digit = true;
        } else if found_digit {
            break;
        }
    }

    num_str.parse().ok()
}
