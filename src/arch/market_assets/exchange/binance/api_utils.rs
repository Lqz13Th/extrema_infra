use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tracing::warn;

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

pub fn binance_fut_inst_to_cli(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let quote_currencies = ["USDT", "USDC", "USD"];
    let (pair, suffix) = match upper.rsplit_once('_') {
        Some((pair, suffix)) => (pair, Some(suffix)),
        None => (upper.as_str(), None),
    };

    for quote in quote_currencies {
        if let Some(base) = pair.strip_suffix(quote) {
            if base.is_empty() {
                warn!("Invalid Binance symbol: {}", symbol);
                return symbol.into();
            }

            return match suffix {
                Some("PERP") => format!("{}_{}_PERP", base, quote),
                Some(expiry) if expiry.chars().all(|c| c.is_ascii_digit()) => {
                    format!("{}_{}_FUT_{}", base, quote, expiry)
                },
                Some(other) => {
                    warn!("Unknown Binance futures suffix: {}", other);
                    upper
                },
                None => format!("{}_{}_PERP", base, quote),
            };
        }
    }

    upper
}

pub fn binance_spot_inst_to_cli(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let quote_currencies = [
        "USDT", "USDC", "USD1", "FDUSD", "TUSD", "USDP", "BUSD", "DAI", "BTC", "ETH", "BNB", "JPY",
        "USD",
    ];

    for quote in quote_currencies {
        if upper.ends_with(quote) {
            let base = &upper[..upper.len() - quote.len()];
            if base.is_empty() {
                warn!("Invalid Binance spot symbol: {}", symbol);
                return symbol.into();
            }
            return format!("{}_{}", base, quote);
        }
    }

    upper
}

pub fn cli_perp_to_pure_lowercase(symbol: &str) -> String {
    cli_um_to_binance_symbol(symbol).to_lowercase()
}

pub fn cli_perp_to_pure_uppercase(symbol: &str) -> String {
    cli_um_to_binance_symbol(symbol).to_uppercase()
}

fn cli_um_to_binance_symbol(symbol: &str) -> String {
    if let Some(cleaned) = symbol.strip_suffix("_PERP") {
        return cleaned.replace("_", "");
    }

    if let Some((pair, expiry)) = symbol.rsplit_once("_FUT_") {
        return format!("{}_{}", pair.replace("_", ""), expiry);
    }

    symbol.replace("_", "")
}

pub fn cli_perp_to_binance_cm(symbol: &str) -> String {
    symbol
        .strip_suffix("_PERP")
        .unwrap_or(symbol)
        .replace("_USDT", "USD")
        .to_string()
}

pub fn cli_spot_to_binance_spot(inst: &str) -> String {
    inst.replace('_', "").to_uppercase()
}
