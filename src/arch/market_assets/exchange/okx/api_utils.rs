use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::error;

use crate::arch::market_assets::base_data::{InstrumentType, SUBSCRIBE_LOWER};
use crate::prelude::{MarginMode, PositionSide};

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
