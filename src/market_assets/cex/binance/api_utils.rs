use serde::Deserialize;
use tracing::error;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct BinanceListenKey {
    pub listenKey: String,
}

pub fn binance_um_to_cli_perp(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    if upper.ends_with("USDT") || upper.ends_with("USDC") {
        let base = &upper[..upper.len() - 4];
        if base.is_empty() {
            error!("Invalid binance um symbol: {}", symbol);
            return symbol.to_string();
        }
        return format!("{}_{}_PERP", base, &upper[upper.len() - 4..]);
    }
    upper
}

pub fn cli_perp_to_pure_lowercase(symbol: &str) -> String {
    let cleaned = symbol.strip_suffix("_PERP").unwrap_or(&symbol);
    cleaned.replace("_", "").to_lowercase()
}

