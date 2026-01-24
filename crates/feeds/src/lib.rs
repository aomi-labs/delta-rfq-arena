//! Mock price feed servers for the OTC RFQ Arena
//!
//! Provides controllable HTTP endpoints that simulate price feeds
//! with configurable behavior (good, stale, malicious).

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::Utc;
use rfq_models::{FeedConfig, PriceUpdate};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State for a mock feed server
#[derive(Debug)]
pub struct FeedState {
    pub config: RwLock<FeedConfig>,
}

impl FeedState {
    pub fn new(config: FeedConfig) -> Self {
        Self {
            config: RwLock::new(config),
        }
    }
}

/// Query parameters for price requests
#[derive(Debug, Deserialize)]
pub struct PriceQuery {
    pub asset: Option<String>,
}

/// Get the current price from a feed
pub async fn get_price(
    State(state): State<Arc<FeedState>>,
    Query(query): Query<PriceQuery>,
) -> Json<PriceUpdate> {
    let config = state.config.read().await;
    let asset = query.asset.unwrap_or_else(|| "dETH".to_string());

    let now = Utc::now();
    let timestamp = if config.force_stale {
        (now.timestamp() as u64).saturating_sub(config.stale_seconds)
    } else {
        now.timestamp() as u64
    };

    let price = if config.is_malicious {
        config.base_price * config.manipulation_factor
    } else {
        // Add small random variance
        let variance = config.base_price * (config.variance_percent / 100.0);
        config.base_price + (rand_variance() * variance)
    };

    Json(PriceUpdate {
        source: config.name.clone(),
        asset,
        price,
        currency: "USDD".to_string(),
        timestamp,
        datetime: chrono::DateTime::from_timestamp(timestamp as i64, 0).unwrap_or(now),
        signature: format!("sig_{}_{}", config.name, timestamp),
    })
}

/// Simple deterministic "random" for demo purposes
fn rand_variance() -> f64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    ((t % 1000) as f64 / 1000.0) - 0.5
}

/// Create a router for a feed
pub fn feed_router(state: Arc<FeedState>) -> Router {
    Router::new()
        .route("/price", get(get_price))
        .with_state(state)
}

/// Run multiple feeds on different ports
pub async fn run_feeds(configs: Vec<(FeedConfig, u16)>) -> anyhow::Result<()> {
    let mut handles = vec![];

    for (config, port) in configs {
        let state = Arc::new(FeedState::new(config));
        let router = feed_router(state);

        let handle = tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                .await
                .unwrap();
            tracing::info!("Feed server listening on port {}", port);
            axum::serve(listener, router).await.unwrap();
        });

        handles.push(handle);
    }

    // Wait for all feeds
    for handle in handles {
        handle.await?;
    }

    Ok(())
}
