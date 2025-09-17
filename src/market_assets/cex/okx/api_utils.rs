use serde::Deserialize;
use tracing::error;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestResOkx<T> {
    pub code: String,
    pub data: Vec<T>,
    pub msg: Option<String>,
}

pub fn get_okx_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");

    let seconds = now.as_secs();
    let millis = now.subsec_millis();

    format!("{}.{}", seconds, millis)
}

pub fn cli_perp_to_okx_swap(symbol: &str) -> String {
    let cleaned = symbol.strip_suffix("_PERP").unwrap_or(symbol);
    format!("{}-SWAP", cleaned.replace("_", "-"))
}

pub fn okx_swap_to_cli(symbol: &str) -> String {
    let parts: Vec<&str> = symbol.split('-').collect();
    match parts.as_slice() {
        [base, quote, kind] if *kind == "SWAP" => format!("{}_{}_PERP", base, quote),
        [base, quote, _] => format!("{}_{}", base, quote),
        [base, quote] => format!("{}_{}", base, quote),
        _ => {
            error!("Invalid okx symbol: {}", symbol);
            symbol.to_string()
        }
    }
}

