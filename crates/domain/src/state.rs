//! Domain state management

use rfq_models::{Quote, QuoteId, FillReceipt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory state for the RFQ domain
#[derive(Debug, Default)]
pub struct DomainState {
    /// Active quotes indexed by ID
    quotes: RwLock<HashMap<QuoteId, Quote>>,
    /// Fill receipts indexed by quote ID
    receipts: RwLock<HashMap<QuoteId, Vec<FillReceipt>>>,
}

impl DomainState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Add a quote
    pub async fn add_quote(&self, quote: Quote) {
        let mut quotes = self.quotes.write().await;
        quotes.insert(quote.id, quote);
    }

    /// Get a quote by ID
    pub async fn get_quote(&self, id: &QuoteId) -> Option<Quote> {
        let quotes = self.quotes.read().await;
        quotes.get(id).cloned()
    }

    /// Get all active quotes
    pub async fn get_active_quotes(&self) -> Vec<Quote> {
        let quotes = self.quotes.read().await;
        quotes.values().filter(|q| q.is_active()).cloned().collect()
    }

    /// Update a quote
    pub async fn update_quote(&self, quote: Quote) {
        let mut quotes = self.quotes.write().await;
        quotes.insert(quote.id, quote);
    }

    /// Add a receipt
    pub async fn add_receipt(&self, quote_id: QuoteId, receipt: FillReceipt) {
        let mut receipts = self.receipts.write().await;
        receipts.entry(quote_id).or_default().push(receipt);
    }

    /// Get receipts for a quote
    pub async fn get_receipts(&self, quote_id: &QuoteId) -> Vec<FillReceipt> {
        let receipts = self.receipts.read().await;
        receipts.get(quote_id).cloned().unwrap_or_default()
    }
}
