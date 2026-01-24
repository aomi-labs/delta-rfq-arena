//! Taker agent implementations

use rfq_models::{FeedEvidence, FillRequest, FillResult, Quote};
use reqwest::Client;

/// Strategy for a taker agent
#[derive(Debug, Clone)]
pub enum TakerStrategy {
    /// Honest taker - uses valid feeds and follows rules
    Honest,
    /// Stale feed attacker - uses outdated price data
    StaleFeed { stale_seconds: u64 },
    /// Spoofed source attacker - uses unauthorized feed
    SpoofedSource { fake_source: String },
    /// Side-payment attacker - tries to add extra transfers
    SidePayment,
    /// Replay attacker - tries to fill multiple times
    Replay { attempts: u32 },
}

/// A taker agent that attempts to fill quotes
pub struct TakerAgent {
    /// HTTP client
    client: Client,
    /// Domain API URL
    domain_url: String,
    /// Taker's owner ID
    owner_id: String,
    /// Taker's shard
    shard: u64,
    /// Taker strategy
    strategy: TakerStrategy,
    /// Feed URLs
    feed_urls: Vec<String>,
}

impl TakerAgent {
    pub fn new(
        domain_url: &str,
        owner_id: &str,
        shard: u64,
        strategy: TakerStrategy,
        feed_urls: Vec<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            domain_url: domain_url.to_string(),
            owner_id: owner_id.to_string(),
            shard,
            strategy,
            feed_urls,
        }
    }

    /// Attempt to fill a quote
    pub async fn fill_quote(&self, quote: &Quote, size: f64, price: f64) -> anyhow::Result<FillResult> {
        let feed_evidence = self.get_feed_evidence(&quote.spec.asset).await?;

        let request = FillRequest {
            taker_owner_id: self.owner_id.clone(),
            taker_shard: self.shard,
            size,
            price,
            feed_evidence,
        };

        let response = self
            .client
            .post(format!("{}/quotes/{}/fill", self.domain_url, quote.id))
            .json(&request)
            .send()
            .await?;

        let result: FillResult = response.json().await?;
        Ok(result)
    }

    /// Get feed evidence based on strategy
    async fn get_feed_evidence(&self, asset: &str) -> anyhow::Result<Vec<FeedEvidence>> {
        match &self.strategy {
            TakerStrategy::Honest => self.get_honest_feeds(asset).await,
            TakerStrategy::StaleFeed { stale_seconds } => {
                self.get_stale_feeds(asset, *stale_seconds).await
            }
            TakerStrategy::SpoofedSource { fake_source } => {
                self.get_spoofed_feeds(asset, fake_source).await
            }
            _ => self.get_honest_feeds(asset).await,
        }
    }

    /// Get honest, fresh feed data
    async fn get_honest_feeds(&self, asset: &str) -> anyhow::Result<Vec<FeedEvidence>> {
        let mut evidence = vec![];

        for url in &self.feed_urls {
            let response: serde_json::Value = self
                .client
                .get(format!("{}/price?asset={}", url, asset))
                .send()
                .await?
                .json()
                .await?;

            evidence.push(FeedEvidence {
                source: response["source"].as_str().unwrap_or("unknown").to_string(),
                asset: asset.to_string(),
                price: response["price"].as_f64().unwrap_or(0.0),
                timestamp: response["timestamp"].as_u64().unwrap_or(0),
                signature: response["signature"].as_str().unwrap_or("").to_string(),
            });
        }

        Ok(evidence)
    }

    /// Get stale feed data (attack)
    async fn get_stale_feeds(&self, asset: &str, stale_seconds: u64) -> anyhow::Result<Vec<FeedEvidence>> {
        let mut evidence = self.get_honest_feeds(asset).await?;

        // Make the first feed stale
        if let Some(first) = evidence.first_mut() {
            first.timestamp = first.timestamp.saturating_sub(stale_seconds);
        }

        Ok(evidence)
    }

    /// Get spoofed feed data (attack)
    async fn get_spoofed_feeds(&self, asset: &str, fake_source: &str) -> anyhow::Result<Vec<FeedEvidence>> {
        let mut evidence = self.get_honest_feeds(asset).await?;

        // Replace first source with fake source (manipulated price)
        if let Some(first) = evidence.first_mut() {
            first.source = fake_source.to_string();
            first.price *= 0.5; // Manipulated lower price
        }

        Ok(evidence)
    }
}
