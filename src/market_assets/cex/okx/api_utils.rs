use serde::Deserialize;
use serde_json::json;
use tracing::error;

use crate::market_assets::base_data::{
    InstrumentType,
    SUBSCRIBE_LOWER
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestResOkx<T> {
    pub code: String,
    pub data: Option<Vec<T>>,
    pub msg: Option<String>,
}

pub fn ws_subscribe_msg_okx(
    channel: &str,
    insts: Option<&[String]>
) -> String {
    let args: Vec<_> = match insts {
        Some(list) => list
            .iter()
            .map(|inst| {
                json!({
                    "channel": channel,
                    "instId": to_okx_inst(inst),
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

pub fn to_okx_inst(symbol: &str) -> String {
    let mut inst = symbol.replace('_', "-");
    if inst.ends_with("-PERP") {
        inst = inst.trim_end_matches("-PERP").to_string() + "-SWAP";
    }
    inst
}


pub fn okx_inst_to_cli(symbol: &str) -> String {
    let parts: Vec<&str> = symbol.split('-').collect();
    match parts.as_slice() {
        [base, quote, kind] if *kind == "SWAP" => format!("{}_{}_PERP", base, quote),
        [base, quote, kind] if kind.chars().all(|c| c.is_numeric()) => {
            format!("{}_{}_FUTURE", base, quote)
        },
        [base, quote] => format!("{}_{}", base, quote),
        _ => {
            error!("Invalid okx symbol: {}", symbol);
            symbol.into()
        },
    }
}


/// Query parameters for retrieving public lead traders from OKX.
/// All fields are optional and can be used to filter or paginate results.
#[derive(Default)]
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

