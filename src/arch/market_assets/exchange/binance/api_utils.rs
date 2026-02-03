use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tracing::error;

use crate::arch::market_assets::base_data::SUBSCRIBE;

use super::api_key::BinanceKey;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct BinanceListenKey {
    pub listenKey: String,
}

fn create_binance_cli_with_key<C, F>(
    keys: HashMap<String, BinanceKey>,
    shared_client: Arc<Client>,
    make_cli: F,
) -> HashMap<String, C>
where
    F: Fn(Arc<Client>, BinanceKey) -> C,
{
    keys.into_iter()
        .map(|(id, key)| (id, make_cli(Arc::clone(&shared_client), key)))
        .collect()
}

pub fn ws_subscribe_msg_binance(param: &str, insts: Option<&[String]>) -> String {
    let params: Vec<String> = match insts {
        Some(list) => list
            .iter()
            .map(|symbol| format!("{}@{}", cli_perp_to_pure_lowercase(symbol), param))
            .collect(),
        None => vec![param.into()],
    };

    let subscribe_msg = json!({
        "method": SUBSCRIBE,
        "params": params,
        "id": 1
    });

    subscribe_msg.to_string()
}

pub fn binance_inst_to_cli(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let quote_currencies = ["USDT", "USDC", "USD"];

    for quote in quote_currencies {
        if upper.ends_with(quote) {
            let base = &upper[..upper.len() - quote.len()];
            if base.is_empty() {
                error!("Invalid Binance symbol: {}", symbol);
                return symbol.into();
            }

            // BTCUSDT_250926
            if let Some((_, expiry)) = upper.split_once('_')
                && expiry.chars().all(|c| c.is_ascii_digit())
            {
                return format!("{}_{}_FUT_{}", base, quote, expiry);
            }

            return format!("{}_{}_PERP", base, quote);
        }
    }

    upper
}

pub fn cli_perp_to_pure_lowercase(symbol: &str) -> String {
    let cleaned = symbol.strip_suffix("_PERP").unwrap_or(symbol);
    cleaned.replace("_", "").to_lowercase()
}

pub fn cli_perp_to_pure_uppercase(symbol: &str) -> String {
    let cleaned = symbol.strip_suffix("_PERP").unwrap_or(symbol);
    cleaned.replace("_", "").to_uppercase()
}

pub fn cli_perp_to_binance_cm(symbol: &str) -> String {
    symbol
        .strip_suffix("_PERP")
        .unwrap_or(symbol)
        .replace("_USDT", "USD")
        .to_string()
}
