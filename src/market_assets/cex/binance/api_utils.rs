use serde::Deserialize;
use serde_json::json;
use tracing::error;

use crate::market_assets::base_data::SUBSCRIBE;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct BinanceListenKey {
    pub listenKey: String,
}

pub fn ws_subscribe_msg_binance(
    param: &str,
    insts: Option<&[String]>
) -> String {
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

pub fn binance_um_to_cli_perp(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    if upper.ends_with("USDT") || upper.ends_with("USDC") {
        let base = &upper[..upper.len() - 4];
        if base.is_empty() {
            error!("Invalid binance um symbol: {}", symbol);
            return symbol.into();
        }
        return format!("{}_{}_PERP", base, &upper[upper.len() - 4..]);
    }
    upper
}

pub fn binance_to_cli(symbol: &str) -> String {
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
            if upper.contains('_') || upper.len() > base.len() + quote.len() {
                return format!("{}_{}_FUTURE", base, quote);
            } else {
                return format!("{}_{}_PERP", base, quote);
            }
        }
    }

    upper
}


pub fn cli_perp_to_pure_lowercase(symbol: &str) -> String {
    let cleaned = symbol.strip_suffix("_PERP").unwrap_or(symbol);
    cleaned.replace("_", "").to_lowercase()
}

