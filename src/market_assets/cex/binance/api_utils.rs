use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
pub struct BinanceListenKey {
    pub listenKey: String,
}

pub fn binance_um_to_perp_symbol(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let len = upper.len();

    if len >= 4 {
        let last4 = &upper[len - 4..];
        if last4 == "USDT" || last4 == "USDC" {
            let base = &upper[..len - 4];
            return format!("{}_{}_PERP", base, last4);
        }
    }

    upper
}

pub fn perp_to_lowercase(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let cleaned = upper.strip_suffix("_PERP").unwrap_or(&upper); 
    cleaned.replace("_", "").to_lowercase()
}

