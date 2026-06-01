use serde::{Deserialize, Serialize};
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

pub fn gate_fut_inst_to_cli(symbol: &str) -> String {
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
    if let Some((pair, expiry)) = symbol.rsplit_once("_FUT_") {
        return format!("{}_{}", pair, expiry).to_uppercase();
    }

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
    match quote {
        "USD" => "btc".to_string(),
        _ => quote.to_lowercase(),
    }
}

pub fn normalize_gate_text(text: &str) -> String {
    if text.starts_with("t-") {
        text.to_string()
    } else {
        format!("t-{}", text)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateDepositHistoryReq {
    pub currency: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl GateDepositHistoryReq {
    pub(crate) fn to_query_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(currency) = self.currency.as_deref() {
            parts.push(format!("currency={}", currency.to_uppercase()));
        }
        if let Some(from) = self.from {
            parts.push(format!("from={from}"));
        }
        if let Some(to) = self.to {
            parts.push(format!("to={to}"));
        }
        if let Some(limit) = self.limit {
            parts.push(format!("limit={limit}"));
        }
        if let Some(offset) = self.offset {
            parts.push(format!("offset={offset}"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("&"))
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateWithdrawHistoryReq {
    pub currency: Option<String>,
    pub withdraw_id: Option<String>,
    pub asset_class: Option<String>,
    pub withdraw_order_id: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl GateWithdrawHistoryReq {
    pub(crate) fn to_query_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(currency) = self.currency.as_deref() {
            parts.push(format!("currency={}", currency.to_uppercase()));
        }
        if let Some(withdraw_id) = self.withdraw_id.as_deref() {
            parts.push(format!("withdraw_id={withdraw_id}"));
        }
        if let Some(asset_class) = self.asset_class.as_deref() {
            parts.push(format!("asset_class={asset_class}"));
        }
        if let Some(woid) = self.withdraw_order_id.as_deref() {
            parts.push(format!("withdraw_order_id={woid}"));
        }
        if let Some(from) = self.from {
            parts.push(format!("from={from}"));
        }
        if let Some(to) = self.to {
            parts.push(format!("to={to}"));
        }
        if let Some(limit) = self.limit {
            parts.push(format!("limit={limit}"));
        }
        if let Some(offset) = self.offset {
            parts.push(format!("offset={offset}"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("&"))
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateWithdrawReq {
    pub currency: String,
    pub address: String,
    pub amount: String,
    pub chain: Option<String>,
    pub memo: Option<String>,
    pub withdraw_order_id: Option<String>,
}

impl GateWithdrawReq {
    pub(crate) fn to_body_string(&self) -> String {
        let mut body = json!({
            "currency": self.currency,
            "address": self.address,
            "amount": self.amount,
        });

        if let Some(chain) = self.chain.as_deref() {
            body["chain"] = json!(chain);
        }
        if let Some(memo) = self.memo.as_deref() {
            body["memo"] = json!(memo);
        }
        if let Some(withdraw_order_id) = self.withdraw_order_id.as_deref() {
            body["withdraw_order_id"] = json!(withdraw_order_id);
        }

        body.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_gate_futures_between_cli_and_native_symbol() {
        assert_eq!(gate_fut_inst_to_cli("BTC_USD"), "BTC_USD_PERP");
        assert_eq!(cli_perp_to_gate_inst("BTC_USD_PERP"), "BTC_USD");
        assert_eq!(
            gate_fut_inst_to_cli("BTC_USD_20241227"),
            "BTC_USD_FUT_20241227"
        );
        assert_eq!(
            cli_perp_to_gate_inst("BTC_USD_FUT_20241227"),
            "BTC_USD_20241227"
        );
    }

    #[test]
    fn infers_gate_settle_for_linear_and_inverse_contracts() {
        assert_eq!(infer_settle_from_inst("ZRX_USDT_PERP"), "usdt");
        assert_eq!(infer_settle_from_inst("BTC_USD_PERP"), "btc");
        assert_eq!(infer_settle_from_inst("BTC_USD_FUT_20241227"), "btc");
    }
}
