use serde::Deserialize;

use crate::arch::{
    market_assets::{
        base_data::{InstrumentType, MarginMode, PositionSide},
        exchange::binance::api_utils::binance_inst_to_cli,
    },
    strategy_base::handler::lob_events::WsAccPosition,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountPositionBinanceUM {
    a: AccountUpdate,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct AccountUpdate {
    P: Vec<AccountPosition>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct AccountPosition {
    s: String,  // Symbol
    pa: String, // Position amount
    ep: String, // Entry price
    mt: String, // Margin type
    ps: String, // Position side ("BOTH", "LONG", "SHORT")
}

impl IntoWsData for WsAccountPositionBinanceUM {
    type Output = Vec<WsAccPosition>;

    fn into_ws(self) -> Vec<WsAccPosition> {
        self.a
            .P
            .into_iter()
            .map(|pos| WsAccPosition {
                inst: binance_inst_to_cli(&pos.s),
                inst_type: if pos.s.contains('_') {
                    InstrumentType::Futures
                } else {
                    InstrumentType::Perpetual
                },
                size: pos.pa.parse().unwrap_or_default(),
                avg_price: pos.ep.parse().unwrap_or_default(),
                position_side: match pos.ps.as_str() {
                    "LONG" => PositionSide::Long,
                    "SHORT" => PositionSide::Short,
                    "BOTH" => PositionSide::Both,
                    _ => PositionSide::Unknown,
                },
                margin_mode: match pos.mt.to_lowercase().as_str() {
                    "cross" => MarginMode::Cross,
                    "isolated" => MarginMode::Isolated,
                    _ => MarginMode::Unknown,
                },
            })
            .collect()
    }
}
