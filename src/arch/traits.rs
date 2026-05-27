//! Core extension traits.
//!
//! Strategy binaries mainly implement [`strategy::Strategy`],
//! [`strategy::CommandEmitter`], and [`strategy::EventHandler`]. Exchange
//! clients implement [`market_lob`] traits to expose public REST, private REST,
//! and websocket message builders. Conversion traits live in [`conversion`] and
//! are used by exchange-specific schemas to normalize raw payloads into shared
//! infra types.

pub mod conversion;
pub mod market_lob;
pub mod strategy;
