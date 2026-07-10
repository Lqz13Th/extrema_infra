use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::arch::{
    market_assets::base_data::{InstrumentType, MarginMode, PositionSide, SUBSCRIBE_LOWER},
    task_execution::task_ws::CandleParam,
};
use crate::errors::InfraResult;

pub fn ws_subscribe_msg_okx(channel: &str, insts: Option<&[String]>) -> String {
    let args: Vec<_> = match insts {
        Some(list) => list
            .iter()
            .map(|inst| {
                json!({
                    "channel": channel,
                    "instId": cli_perp_to_okx_inst(inst),
                })
            })
            .collect(),
        None => vec![json!({ "channel": channel })],
    };

    let subscribe_msg = json!({
        "op": SUBSCRIBE_LOWER,
        "args": args
    });

    subscribe_msg.to_string()
}

pub fn get_okx_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");

    let seconds = now.as_secs();
    let millis = now.subsec_millis();

    format!("{}.{}", seconds, millis)
}

pub fn cli_perp_to_okx_inst(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    if let Some((pair, expiry)) = upper.rsplit_once("_FUT_") {
        return format!("{}-{}", pair.replace('_', "-"), expiry);
    }

    if let Some(pair) = upper.strip_suffix("_PERP") {
        return format!("{}-SWAP", pair.replace('_', "-"));
    }

    upper.replace('_', "-")
}

pub fn cli_inst_to_okx_inst(symbol: &str, inst_type: &InstrumentType) -> InfraResult<String> {
    let upper = symbol.to_uppercase();
    match inst_type {
        InstrumentType::Spot => Ok(upper.replace('_', "-")),
        InstrumentType::Perpetual => {
            if upper.ends_with("-SWAP") {
                Ok(upper)
            } else if let Some(pair) = upper.strip_suffix("_PERP") {
                Ok(format!("{}-SWAP", pair.replace('_', "-")))
            } else {
                Ok(format!("{}-SWAP", upper.replace('_', "-")))
            }
        },
        InstrumentType::Futures => Ok(cli_perp_to_okx_inst(&upper)),
        InstrumentType::Options => Ok(upper.replace('_', "-")),
        InstrumentType::Unknown => Err(crate::errors::InfraError::ApiCliError(
            "Unknown instrument type for OKX instId conversion".into(),
        )),
    }
}

pub fn okx_inst_to_cli(symbol: &str) -> String {
    let parts: Vec<&str> = symbol.split('-').collect();
    match parts.as_slice() {
        [base, quote, kind] if *kind == "SWAP" => format!("{}_{}_PERP", base, quote),
        [base, quote, expiry]
            if expiry.len() == 6 && expiry.chars().all(|c| c.is_ascii_digit()) =>
        {
            format!("{}_{}_FUT_{}", base, quote, expiry)
        },
        [base, quote] => format!("{}_{}", base, quote),
        _ => {
            error!("Invalid okx symbol: {}", symbol);
            symbol.into()
        },
    }
}

pub fn okx_preopen_inst(symbol: &str) -> Option<(InstrumentType, String)> {
    let (inst_type, inst) = symbol.strip_prefix("LISTING-")?.split_once('-')?;
    if !inst.contains('-') {
        return None;
    }

    match inst_type {
        "SPOT" => Some((InstrumentType::Spot, inst.to_string())),
        "SWAP" => Some((InstrumentType::Perpetual, format!("{inst}-SWAP"))),
        "FUTURES" => Some((InstrumentType::Futures, inst.to_string())),
        "OPTION" => Some((InstrumentType::Options, inst.to_string())),
        _ => None,
    }
}

pub fn okx_candle_interval(interval: &CandleParam) -> &str {
    match interval {
        CandleParam::OneSecond => "1s",
        CandleParam::OneMinute => "1m",
        CandleParam::FiveMinutes => "5m",
        CandleParam::FifteenMinutes => "15m",
        CandleParam::OneHour => "1H",
        CandleParam::FourHours => "4H",
        CandleParam::OneDay => "1Dutc",
        CandleParam::OneWeek => "1W",
        CandleParam::Custom(value) => value.as_str(),
    }
}

/// Query parameters for retrieving public lead traders from OKX.
/// All fields are optional and can be used to filter or paginate results.
#[derive(Clone, Debug, Default)]
pub struct PubLeadTraderQuery {
    /// Instrument type: Spot / Perpetual / Option
    pub inst_type: Option<InstrumentType>,
    /// Sorting type: "overview" / "pnl" / "aum" / "win_ratio" / "pnl_ratio" / "current_copy_trader_pnl".
    pub sort_type: Option<String>,
    /// Trader state: 0 = all, 1 = has vacancies
    pub state: Option<u64>,
    /// Minimum leading days (1 = 7 days, 2 = 30 days, 3 = 90 days, 4 = 180 days, etc.)
    pub min_lead_days: Option<u64>,
    /// Minimum assets under management
    pub min_assets: Option<f64>,
    /// Maximum assets under management
    pub max_assets: Option<f64>,
    /// Minimum AUM (Assets Under Management)
    pub min_aum: Option<f64>,
    /// Maximum AUM
    pub max_aum: Option<f64>,
    /// Data version, e.g., timestamp string "20250918140000"
    pub data_ver: Option<String>,
    /// Page number for pagination
    pub page: Option<u64>,
    /// Number of records per page
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LeadtraderSubposition {
    pub timestamp: u64,
    pub unique_code: String,
    pub inst: String,
    pub subpos_id: String,
    pub pos_side: PositionSide,
    pub margin_mode: MarginMode,
    pub leverage: f64,
    pub open_ts: u64,
    pub open_avg_price: f64,
    pub size: f64,
    pub ins_type: InstrumentType,
    pub margin: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LeadtraderSubpositionHistory {
    pub timestamp: u64,
    pub unique_code: String,
    pub inst: String,
    pub subpos_id: String,
    pub pos_side: PositionSide,
    pub margin_mode: MarginMode,
    pub ins_type: InstrumentType,
    pub leverage: f64,
    pub size: f64,
    pub margin: f64,
    pub open_ts: u64,
    pub open_avg_price: f64,
    pub close_ts: u64,
    pub close_avg_price: f64,
    pub pnl: f64,
    pub pnl_ratio: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CurrentLeadtrader {
    pub timestamp: u64,
    pub unique_code: String,
    pub nick_name: String,
    pub margin: f64,
    pub copy_pnl: f64,
    pub copy_amount: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PubLeadtraderInfo {
    pub data_version: u64,
    pub total_page: u64,
    pub pub_leadtraders: Vec<PubLeadtrader>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PubLeadtrader {
    pub unique_code: String,
    pub nick_name: String,
    pub aum: f64,
    pub copy_state: u64,
    pub copy_trader_num: u64,
    pub max_copy_trader_num: u64,
    pub accum_copy_trader_num: u64,
    pub lead_days: u64,
    pub win_ratio: f64,
    pub pnl_ratio: f64,
    pub pnl: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PubLeadtraderStats {
    pub timestamp: u64,
    pub win_ratio: f64,
    pub profit_days: u64,
    pub loss_days: f64,
    pub invest_amount: f64,
    pub avg_sub_pos_national: f64,
    pub current_copy_trader_pnl: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OkxAssetWithdrawalHistoryReq {
    pub ccy: Option<String>,
    pub wd_id: Option<String>,
    pub client_id: Option<String>,
    pub tx_id: Option<String>,
    pub withdrawal_type: Option<String>,
    pub state: Option<String>,
    pub after: Option<u64>,
    pub before: Option<u64>,
    pub limit: Option<u32>,
}

impl OkxAssetWithdrawalHistoryReq {
    pub(crate) fn to_query_body(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(c) = self.ccy.as_deref() {
            parts.push(format!("ccy={}", c.to_ascii_uppercase()));
        }
        if let Some(id) = self.wd_id.as_deref() {
            parts.push(format!("wdId={id}"));
        }
        if let Some(id) = self.client_id.as_deref() {
            parts.push(format!("clientId={id}"));
        }
        if let Some(tx) = self.tx_id.as_deref() {
            parts.push(format!("txId={tx}"));
        }
        if let Some(t) = self.withdrawal_type.as_deref() {
            parts.push(format!("type={t}"));
        }
        if let Some(s) = self.state.as_deref() {
            parts.push(format!("state={s}"));
        }
        if let Some(a) = self.after {
            parts.push(format!("after={a}"));
        }
        if let Some(b) = self.before {
            parts.push(format!("before={b}"));
        }
        if let Some(l) = self.limit {
            parts.push(format!("limit={l}"));
        }

        parts.join("&")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OkxAssetWithdrawalReq {
    pub ccy: String,
    pub amt: String,
    pub dest: String,
    pub to_addr: String,
    pub fee: String,
    pub chain: Option<String>,
    pub area_code: Option<String>,
    pub client_id: Option<String>,
    pub fee_ccy: Option<String>,
    pub addr_label: Option<String>,
    pub ln_invoice: Option<String>,
}

impl OkxAssetWithdrawalReq {
    pub(crate) fn to_json_body(&self) -> String {
        let mut body = json!({
            "ccy": self.ccy.to_ascii_uppercase(),
            "amt": self.amt,
            "dest": self.dest,
            "toAddr": self.to_addr,
            "fee": self.fee,
        });

        if let Some(c) = self.chain.as_deref() {
            body["chain"] = json!(c);
        }
        if let Some(a) = self.area_code.as_deref() {
            body["areaCode"] = json!(a);
        }
        if let Some(c) = self.client_id.as_deref() {
            body["clientId"] = json!(c);
        }
        if let Some(c) = self.fee_ccy.as_deref() {
            body["feeCcy"] = json!(c);
        }
        if let Some(l) = self.addr_label.as_deref() {
            body["addrLabel"] = json!(l);
        }
        if let Some(l) = self.ln_invoice.as_deref() {
            body["lnInvoice"] = json!(l);
        }

        body.to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OkxAssetTransferReq {
    pub ccy: String,
    pub amt: String,
    pub from: String,
    pub to: String,
    pub sub_acct: Option<String>,
    pub transfer_type: Option<String>,
    pub loan_trans: Option<bool>,
    pub omit_pos_risk: Option<bool>,
    pub client_id: Option<String>,
}

impl OkxAssetTransferReq {
    pub(crate) fn to_json_body(&self) -> String {
        let mut body = json!({
            "ccy": self.ccy.to_ascii_uppercase(),
            "amt": self.amt,
            "from": self.from,
            "to": self.to,
        });

        if let Some(s) = self.sub_acct.as_deref() {
            body["subAcct"] = json!(s);
        }
        if let Some(t) = self.transfer_type.as_deref() {
            body["type"] = json!(t);
        }
        if let Some(b) = self.loan_trans {
            body["loanTrans"] = json!(b);
        }
        if let Some(b) = self.omit_pos_risk {
            body["omitPosRisk"] = json!(b);
        }
        if let Some(c) = self.client_id.as_deref() {
            body["clientId"] = json!(c);
        }

        body.to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OkxAssetDepositHistoryReq {
    pub ccy: Option<String>,
    pub dep_id: Option<String>,
    pub from_wd_id: Option<String>,
    pub tx_id: Option<String>,
    pub deposit_type: Option<String>,
    pub state: Option<String>,
    pub after: Option<u64>,
    pub before: Option<u64>,
    pub limit: Option<u32>,
}

impl OkxAssetDepositHistoryReq {
    pub(crate) fn to_query_body(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(ccy) = self.ccy.as_deref() {
            parts.push(format!("ccy={}", ccy.to_ascii_uppercase()));
        }
        if let Some(id) = self.dep_id.as_deref() {
            parts.push(format!("depId={id}"));
        }
        if let Some(id) = self.from_wd_id.as_deref() {
            parts.push(format!("fromWdId={id}"));
        }
        if let Some(tx) = self.tx_id.as_deref() {
            parts.push(format!("txId={tx}"));
        }
        if let Some(t) = self.deposit_type.as_deref() {
            parts.push(format!("type={t}"));
        }
        if let Some(s) = self.state.as_deref() {
            parts.push(format!("state={s}"));
        }
        if let Some(after) = self.after {
            parts.push(format!("after={after}"));
        }
        if let Some(before) = self.before {
            parts.push(format!("before={before}"));
        }
        if let Some(limit) = self.limit {
            parts.push(format!("limit={limit}"));
        }

        parts.join("&")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_okx_instruments_between_cli_and_native_symbol() {
        assert_eq!(okx_inst_to_cli("BTC-USDT-SWAP"), "BTC_USDT_PERP");
        assert_eq!(cli_perp_to_okx_inst("BTC_USDT_PERP"), "BTC-USDT-SWAP");
        assert_eq!(okx_inst_to_cli("BTC-USD-240329"), "BTC_USD_FUT_240329");
        assert_eq!(cli_perp_to_okx_inst("BTC_USD_FUT_240329"), "BTC-USD-240329");
        assert_eq!(
            cli_inst_to_okx_inst("BTC_USDT", &InstrumentType::Spot).unwrap(),
            "BTC-USDT"
        );
        assert_eq!(
            cli_inst_to_okx_inst("BTC_USDT", &InstrumentType::Perpetual).unwrap(),
            "BTC-USDT-SWAP"
        );
        assert_eq!(
            cli_inst_to_okx_inst("BTC_USD_FUT_240329", &InstrumentType::Futures).unwrap(),
            "BTC-USD-240329"
        );
    }

    #[test]
    fn parses_okx_preopen_listing_symbol() {
        assert_eq!(
            okx_preopen_inst("LISTING-SPOT-SLX-USDT"),
            Some((InstrumentType::Spot, "SLX-USDT".into()))
        );
        assert_eq!(
            okx_preopen_inst("LISTING-SWAP-SLX-USDT"),
            Some((InstrumentType::Perpetual, "SLX-USDT-SWAP".into()))
        );
        assert_eq!(okx_preopen_inst("SLX-USDT"), None);
    }
}
