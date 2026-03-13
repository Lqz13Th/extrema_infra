
pub const HYPERLIQUID_QUOTE: &str = "USDC";
pub const HYPERLIQUID_SPOT_ASSET_OFFSET: u32 = 10_000;

pub fn hyperliquid_perp_asset_id(index: usize) -> String {
    index.to_string()
}

pub fn hyperliquid_spot_asset_id(index: u32) -> String {
    (HYPERLIQUID_SPOT_ASSET_OFFSET + index).to_string()
}

pub fn hyperliquid_perp_to_cli(symbol: &str) -> String {
    format!("{}_{}_PERP", symbol, HYPERLIQUID_QUOTE)
}

pub fn hyperliquid_spot_to_cli(symbol: &str, base: &str, quote: &str) -> String {
    if symbol.contains('/') {
        symbol.replace('/', "_")
    } else {
        format!("{}_{}", base, quote)
    }
}
