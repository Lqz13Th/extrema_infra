use tracing::error;

pub fn gate_inst_to_cli(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let parts: Vec<&str> = upper.split('_').collect();
    match parts.as_slice() {
        [base, quote] => format!("{}_{}_PERP", base, quote),
        [base, quote, expiry]
            if expiry.len() == 8 && expiry.chars().all(|c| c.is_ascii_digit()) =>
        {
            format!("{}_{}_FUT_{}", base, quote, expiry)
        },
        _ => {
            error!("Invalid Gate symbol: {}", symbol);
            symbol.into()
        },
    }
}

pub fn cli_perp_to_gate_inst(symbol: &str) -> String {
    let cleaned = symbol
        .strip_suffix("_PERP")
        .or_else(|| symbol.strip_suffix("_FUTURE"))
        .unwrap_or(symbol);
    cleaned.to_uppercase()
}
