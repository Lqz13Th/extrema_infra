use tracing::error;

pub fn gate_inst_to_cli(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    let parts: Vec<&str> = upper.split('_').collect();
    match parts.as_slice() {
        [base, quote] => format!("{}_{}_PERP", base, quote),
        [base, quote, kind] if *kind == "PERP" => format!("{}_{}_PERP", base, quote),
        [base, quote, kind] if *kind == "FUTURE" => format!("{}_{}_FUTURE", base, quote),
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
