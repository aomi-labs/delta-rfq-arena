//! Core models for the OTC RFQ Arena
//!
//! This crate defines the data structures used throughout the RFQ system:
//! - Quotes and their specifications
//! - Guardrails (constraints) compiled from English
//! - Fill attempts and results
//! - Price feed data
//!
//! ## Features
//!
//! - `std` (default): Standard library support with full functionality
//! - Without `std`: Minimal build for zkVM environments

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

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
