use serde_json::json;
use tracing::error;

use crate::arch::market_assets::{api_general::get_seconds_timestamp, base_data::SUBSCRIBE_LOWER};
use crate::errors::{InfraError, InfraResult};

pub fn ws_subscribe_msg_gate_futures(channel: &str, payload: Vec<String>) -> String {
    let msg = json!({
        "time": get_seconds_timestamp(),
        "channel": channel,
        "event": SUBSCRIBE_LOWER,
        "payload": payload,
    });

    msg.to_string()
}

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

pub fn gate_first_contract(insts: Option<&[String]>) -> InfraResult<String> {
    let inst = insts
        .and_then(|list| list.first())
        .ok_or_else(|| InfraError::ApiCliError("Gate futures ws requires one instrument".into()))?;
    Ok(cli_perp_to_gate_inst(inst))
}

pub fn gate_contracts_from_insts(insts: Option<&[String]>) -> InfraResult<Vec<String>> {
    let list = insts.ok_or_else(|| {
        InfraError::ApiCliError("Gate futures ws requires instrument list".into())
    })?;
    if list.is_empty() {
        return Err(InfraError::ApiCliError(
            "Gate futures ws requires instrument list".into(),
        ));
    }
    Ok(list.iter().map(|s| cli_perp_to_gate_inst(s)).collect())
}

pub fn infer_settle_from_inst(inst: &str) -> String {
    let gate_inst = cli_perp_to_gate_inst(inst);
    let parts: Vec<&str> = gate_inst.split('_').collect();
    let quote = parts.get(1).copied().unwrap_or("USDT");
    quote.to_lowercase()
}

pub fn normalize_gate_text(text: &str) -> String {
    if text.starts_with("t-") {
        text.to_string()
    } else {
        format!("t-{}", text)
    }
}
