//! Agent implementations for the OTC RFQ Arena
//!
//! Provides automated agents for:
//! - Makers: post quotes, respond to fills
//! - Takers: attempt fills (honest and adversarial)

pub mod maker;
pub mod taker;

pub use maker::MakerAgent;
pub use taker::{TakerAgent, TakerStrategy};
