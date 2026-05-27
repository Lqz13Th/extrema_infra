//! Normalized market data, account data, and exchange clients.
//!
//! Exchange APIs differ in instrument names, position-side conventions, order
//! fields, timestamp units, websocket payload shapes, and authentication flows.
//! The types in this module provide the common representation that strategy
//! modules consume after exchange-specific clients parse and normalize raw
//! responses.
//!
//! Use [`market_core::Market`] to identify venues, [`base_data`] for shared
//! enums such as order side and instrument type, and [`api_data`] for normalized
//! REST payloads. Built-in exchange clients live under [`exchange`] and are
//! enabled with crate features such as `binance`, `okx`, `gate`, and
//! `hyperliquid`.

pub mod api_data;
pub mod exchange;

pub mod api_general;
pub mod base_data;
pub mod market_core;
