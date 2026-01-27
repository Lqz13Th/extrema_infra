use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        base_data::{InstrumentType, MarginMode, PositionSide},
        exchange::okx::api_utils::okx_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::{WsAccBalPos, WsAccBalance, WsAccPosition},
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsBalAndPosOkx {
    pTime: String,
    eventType: String,
    balData: Vec<AccountBalance>,
    posData: Vec<AccountPosition>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct AccountBalance {
    ccy: String,
    cashBal: String,
    uTime: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct AccountPosition {
    posId: String,
    tradeId: Option<String>,
    instId: String,
    instType: String,
    mgnMode: String,
    posSide: String,
    pos: String,
    ccy: String,
    posCcy: String,
    avgPx: String,
    nonSettleAvgPx: String,
    settledPnl: String,
    uTime: String,
}

impl IntoWsData for WsBalAndPosOkx {
    type Output = WsAccBalPos;
    fn into_ws(self) -> WsAccBalPos {
        let balances = self
            .balData
            .into_iter()
            .map(|b| WsAccBalance {
                inst: b.ccy,
                balance: b.cashBal.parse().unwrap_or_default(),
            })
            .collect();

        let positions = self
            .posData
            .into_iter()
            .map(|p| WsAccPosition {
                inst: okx_inst_to_cli(&p.instId),
                inst_type: match p.instType.as_str() {
                    "SWAP" => InstrumentType::Perpetual,
                    "FUTURES" => InstrumentType::Futures,
                    "SPOT" => InstrumentType::Spot,
                    _ => InstrumentType::Unknown,
                },
                size: p.pos.parse().unwrap_or_default(),
                avg_price: p.avgPx.parse().unwrap_or_default(),
                position_side: match p.posSide.as_str() {
                    "long" => PositionSide::Long,
                    "short" => PositionSide::Short,
                    "net" => PositionSide::Both,
                    _ => PositionSide::Unknown,
                },
                margin_mode: match p.mgnMode.to_lowercase().as_str() {
                    "cross" => MarginMode::Cross,
                    "isolated" => MarginMode::Isolated,
                    _ => MarginMode::Unknown,
                },
            })
            .collect();

        WsAccBalPos {
            timestamp: ts_to_micros(self.pTime.parse().unwrap_or_default()),
            market: Market::Okx,
            event: self.eventType,
            balances,
            positions,
        }
    }
}
