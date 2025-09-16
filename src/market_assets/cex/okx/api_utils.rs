use serde::Deserialize;
use tracing::error;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestResOkx<T> {
    pub code: String,
    pub data: Vec<T>,
    pub msg: Option<String>,
}


pub fn cli_perp_to_okx_swap(symbol: &str) -> String {
    let cleaned = symbol.strip_suffix("_PERP").unwrap_or(&symbol);
    format!("{}-SWAP", cleaned.replace("_", "-"))
}

pub fn okx_swap_to_cli_perp(symbol: &str) -> String {
    let parts: Vec<&str> = symbol.split('-').collect();
    if parts.len() != 3 || parts[2] != "SWAP" {
        error!("Invalid okx swap symbol: {}", symbol);
        return symbol.to_string();
    }
    format!("{}_{}_PERP", parts[0], parts[1])
}
