//! Core extension traits.
//!
//! Strategy binaries mainly implement [`strategy::Strategy`],
//! [`strategy::CommandEmitter`], and [`strategy::EventHandler`]. Exchange
//! clients implement [`market_lob`] traits to expose public REST, private REST,
//! and websocket message builders; venues implemented outside this crate also
//! implement its websocket frame decoder. Conversion traits live in [`conversion`] and
//! normalize raw payloads into shared infra types; exchange-specific schemas and
//! custom-venue decoders implement them.

pub mod conversion;
pub mod market_lob;
pub mod strategy;
