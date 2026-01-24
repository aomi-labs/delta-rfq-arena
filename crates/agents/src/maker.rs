//! Maker agent implementation

use rfq_models::{CreateQuoteRequest, Quote};
use reqwest::Client;

/// A maker agent that posts quotes
pub struct MakerAgent {
    /// HTTP client
    client: Client,
    /// Domain API URL
    domain_url: String,
    /// Maker's owner ID
    owner_id: String,
    /// Maker's shard
    shard: u64,
}

impl MakerAgent {
    pub fn new(domain_url: &str, owner_id: &str, shard: u64) -> Self {
        Self {
            client: Client::new(),
            domain_url: domain_url.to_string(),
            owner_id: owner_id.to_string(),
            shard,
        }
    }

    /// Post a new quote
    pub async fn post_quote(&self, text: &str) -> anyhow::Result<Quote> {
        let request = CreateQuoteRequest {
            text: text.to_string(),
            maker_owner_id: self.owner_id.clone(),
            maker_shard: self.shard,
        };

        let response = self
            .client
            .post(format!("{}/quotes", self.domain_url))
            .json(&request)
            .send()
            .await?;

        let quote: Quote = response.json().await?;
        Ok(quote)
    }

    /// Cancel a quote
    pub async fn cancel_quote(&self, quote_id: &str) -> anyhow::Result<()> {
        self.client
            .delete(format!("{}/quotes/{}", self.domain_url, quote_id))
            .send()
            .await?;
        Ok(())
    }
}
