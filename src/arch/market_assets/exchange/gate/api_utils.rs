use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{error, warn};

use crate::arch::market_assets::{api_general::get_seconds_timestamp, base_data::SUBSCRIBE_LOWER};
use crate::arch::task_execution::task_ws::LobFrequency;
use crate::errors::{InfraError, InfraResult};

pub const GATE_CHANNEL_ID_EXTRA_KEY: &str = "gate_channel_id";
pub const GATE_CHANNEL_ID_HEADER: &str = "X-Gate-Channel-Id";
pub const GATE_SIZE_DECIMAL_HEADER: &str = "X-Gate-Size-Decimal";
pub const GATE_SIZE_DECIMAL_HEADER_VALUE: &str = "1";

pub(crate) fn value_to_order_id(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(id)) if !id.is_empty() && id != "-" => Some(id.clone()),
        Some(Value::Number(id)) => Some(id.to_string()),
        _ => None,
    }
}

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
    let list = insts
        .ok_or_else(|| InfraError::ApiCliError("Gate futures ws requires one instrument".into()))?;
    if list.len() > 1 {
        warn!(
            "Gate futures ws supports one instrument for this channel; got {} instruments: {:?}; using the first one",
            list.len(),
            list
        );
    }

    let inst = list
        .first()
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

pub(crate) fn gate_lob_depth(depth: &Option<u16>) -> InfraResult<u16> {
    match depth.as_ref().copied() {
        None => Ok(20),
        Some(depth @ (20 | 50 | 100)) => Ok(depth),
        Some(depth) => Err(InfraError::ApiCliError(format!(
            "Gate futures LOB supports only 20, 50, or 100 levels: {}",
            depth
        ))),
    }
}

pub(crate) fn nonzero_depth_u16(depth: usize) -> InfraResult<Option<u16>> {
    if depth == 0 {
        return Ok(None);
    }

    u16::try_from(depth)
        .map(Some)
        .map_err(|_| InfraError::ApiCliError(format!("Depth exceeds u16 range: {}", depth)))
}

pub(crate) fn gate_lob_bbo_frequency(frequency: &Option<LobFrequency>) -> InfraResult<()> {
    match frequency {
        None | Some(LobFrequency::Realtime) => Ok(()),
        Some(freq) => Err(InfraError::ApiCliError(format!(
            "Gate futures book ticker does not support requested frequency: {:?}",
            freq
        ))),
    }
}

pub(crate) fn gate_lob_snapshot_frequency(frequency: &Option<LobFrequency>) -> InfraResult<()> {
    match frequency {
        None => Ok(()),
        Some(freq) => Err(InfraError::ApiCliError(format!(
            "Gate futures order book snapshot does not support frequency: {:?}",
            freq
        ))),
    }
}

pub(crate) fn gate_lob_update_frequency(
    frequency: &Option<LobFrequency>,
    depth: u16,
) -> InfraResult<&'static str> {
    match frequency {
        None | Some(LobFrequency::Ms100) => Ok("100ms"),
        Some(LobFrequency::Ms20) if depth == 20 => Ok("20ms"),
        Some(LobFrequency::Ms20) => Err(InfraError::ApiCliError(format!(
            "Gate futures 20ms LOB updates support only 20 levels, got {}",
            depth
        ))),
        Some(freq) => Err(InfraError::ApiCliError(format!(
            "Gate futures LOB updates support only 20ms or 100ms frequency: {:?}",
            freq
        ))),
    }
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

pub fn take_gate_channel_id(extra: &mut HashMap<String, String>) -> InfraResult<Option<String>> {
    let Some(channel_id) = extra.remove(GATE_CHANNEL_ID_EXTRA_KEY) else {
        return Ok(None);
    };

    validate_gate_channel_id(&channel_id)?;
    Ok(Some(channel_id))
}

fn validate_gate_channel_id(channel_id: &str) -> InfraResult<()> {
    if channel_id.is_empty()
        || channel_id.len() >= 20
        || !channel_id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(InfraError::ApiCliError(format!(
            "Invalid Gate broker channel id: {channel_id}; expected <20 lowercase letters/digits"
        )));
    }

    Ok(())
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateSubAccountTransferAccountType {
    Spot,
    Futures,
    CrossMargin,
    Delivery,
    Options,
}

impl GateSubAccountTransferAccountType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Spot => "spot",
            Self::Futures => "futures",
            Self::CrossMargin => "cross_margin",
            Self::Delivery => "delivery",
            Self::Options => "options",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateSubAccountToSubAccountTransferAccountType {
    Spot,
    Futures,
    Delivery,
}

impl GateSubAccountToSubAccountTransferAccountType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Spot => "spot",
            Self::Futures => "futures",
            Self::Delivery => "delivery",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateSubAccountTransferDirection {
    To,
    From,
}

impl GateSubAccountTransferDirection {
    pub fn as_str(&self) -> &str {
        match self {
            Self::To => "to",
            Self::From => "from",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateSubAccountTransferReq {
    pub sub_account: String,
    pub sub_account_type: Option<GateSubAccountTransferAccountType>,
    pub currency: String,
    pub amount: String,
    pub direction: GateSubAccountTransferDirection,
    pub client_order_id: Option<String>,
}

impl GateSubAccountTransferReq {
    pub(crate) fn to_body_string(&self) -> String {
        let mut body = json!({
            "sub_account": self.sub_account,
            "currency": self.currency.to_uppercase(),
            "amount": self.amount,
            "direction": self.direction.as_str(),
        });

        if let Some(sub_account_type) = self.sub_account_type.as_ref() {
            body["sub_account_type"] = json!(sub_account_type.as_str());
        }
        if let Some(client_order_id) = self.client_order_id.as_deref() {
            body["client_order_id"] = json!(client_order_id);
        }

        body.to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateSubAccountTransferHistoryReq {
    pub sub_uid: Option<String>,
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl GateSubAccountTransferHistoryReq {
    pub(crate) fn to_query_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(sub_uid) = self.sub_uid.as_deref() {
            parts.push(format!("sub_uid={sub_uid}"));
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateSubAccountToSubAccountTransferReq {
    pub currency: String,
    pub sub_account_from: String,
    pub sub_account_from_type: GateSubAccountToSubAccountTransferAccountType,
    pub sub_account_to: String,
    pub sub_account_to_type: GateSubAccountToSubAccountTransferAccountType,
    pub amount: String,
}

impl GateSubAccountToSubAccountTransferReq {
    pub(crate) fn to_body_string(&self) -> String {
        json!({
            "currency": self.currency.to_uppercase(),
            "sub_account_from": self.sub_account_from,
            "sub_account_from_type": self.sub_account_from_type.as_str(),
            "sub_account_to": self.sub_account_to,
            "sub_account_to_type": self.sub_account_to_type.as_str(),
            "amount": self.amount,
        })
        .to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateTransferOrderStatusReq {
    pub client_order_id: Option<String>,
    pub tx_id: Option<String>,
}

impl GateTransferOrderStatusReq {
    pub(crate) fn to_query_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(client_order_id) = self.client_order_id.as_deref() {
            parts.push(format!("client_order_id={client_order_id}"));
        }
        if let Some(tx_id) = self.tx_id.as_deref() {
            parts.push(format!("tx_id={tx_id}"));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("&"))
        }
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

    #[test]
    fn takes_and_validates_gate_channel_id() {
        let mut extra = HashMap::from([
            (
                GATE_CHANNEL_ID_EXTRA_KEY.to_string(),
                "broker123".to_string(),
            ),
            ("account".to_string(), "spot".to_string()),
        ]);

        assert_eq!(
            take_gate_channel_id(&mut extra).unwrap(),
            Some("broker123".to_string())
        );
        assert!(!extra.contains_key(GATE_CHANNEL_ID_EXTRA_KEY));
        assert_eq!(extra.get("account").map(String::as_str), Some("spot"));
    }

    #[test]
    fn rejects_invalid_gate_channel_id() {
        for channel_id in ["", "Broker123", "broker_123", "abcdefghijklmnopqrst"] {
            let mut extra = HashMap::from([(
                GATE_CHANNEL_ID_EXTRA_KEY.to_string(),
                channel_id.to_string(),
            )]);
            assert!(take_gate_channel_id(&mut extra).is_err());
        }
    }

    #[test]
    fn builds_gate_sub_account_transfer_body() {
        let req = GateSubAccountTransferReq {
            sub_account: "10001".into(),
            sub_account_type: Some(GateSubAccountTransferAccountType::Futures),
            currency: "usdt".into(),
            amount: "10".into(),
            direction: GateSubAccountTransferDirection::To,
            client_order_id: Some("order-1".into()),
        };

        let body: serde_json::Value = serde_json::from_str(&req.to_body_string()).unwrap();
        assert_eq!(body["sub_account"], "10001");
        assert_eq!(body["sub_account_type"], "futures");
        assert_eq!(body["currency"], "USDT");
        assert_eq!(body["amount"], "10");
        assert_eq!(body["direction"], "to");
        assert_eq!(body["client_order_id"], "order-1");
    }

    #[test]
    fn builds_gate_sub_account_to_sub_account_transfer_body() {
        let req = GateSubAccountToSubAccountTransferReq {
            currency: "btc".into(),
            sub_account_from: "10001".into(),
            sub_account_from_type: GateSubAccountToSubAccountTransferAccountType::Spot,
            sub_account_to: "10002".into(),
            sub_account_to_type: GateSubAccountToSubAccountTransferAccountType::Delivery,
            amount: "0.1".into(),
        };

        let body: serde_json::Value = serde_json::from_str(&req.to_body_string()).unwrap();
        assert_eq!(body["currency"], "BTC");
        assert_eq!(body["sub_account_from"], "10001");
        assert_eq!(body["sub_account_from_type"], "spot");
        assert_eq!(body["sub_account_to"], "10002");
        assert_eq!(body["sub_account_to_type"], "delivery");
        assert_eq!(body["amount"], "0.1");
    }

    #[test]
    fn builds_gate_transfer_query_strings() {
        assert_eq!(
            GateSubAccountTransferAccountType::CrossMargin.as_str(),
            "cross_margin"
        );
        assert_eq!(
            GateSubAccountTransferAccountType::Options.as_str(),
            "options"
        );
        assert_eq!(
            GateSubAccountToSubAccountTransferAccountType::Futures.as_str(),
            "futures"
        );

        let history_req = GateSubAccountTransferHistoryReq {
            sub_uid: Some("10001".into()),
            from: Some(1),
            to: Some(2),
            limit: Some(3),
            offset: Some(4),
        };
        assert_eq!(
            history_req.to_query_string().as_deref(),
            Some("sub_uid=10001&from=1&to=2&limit=3&offset=4")
        );

        let status_req = GateTransferOrderStatusReq {
            client_order_id: Some("order-1".into()),
            tx_id: Some("tx-1".into()),
        };
        assert_eq!(
            status_req.to_query_string().as_deref(),
            Some("client_order_id=order-1&tx_id=tx-1")
        );
    }
}
