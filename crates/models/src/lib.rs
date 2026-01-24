//! Core models for the OTC RFQ Arena
//!
//! This crate defines the data structures used throughout the RFQ system:
//! - Quotes and their specifications
//! - Guardrails (constraints) compiled from English
//! - Fill attempts and results
//! - Price feed data

mod quote;
mod constraints;
mod fill;
mod feed;
mod receipt;

pub use quote::*;
pub use constraints::*;
pub use fill::*;
pub use feed::*;
pub use receipt::*;
