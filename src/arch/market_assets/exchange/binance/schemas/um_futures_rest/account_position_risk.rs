use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::PositionData,
    api_general::ts_to_micros,
    base_data::{InstrumentType, PositionSide},
    exchange::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestAccountPosRiskBinanceUM {
    pub symbol: String,
    pub positionSide: String,
    pub positionAmt: String,
    pub entryPrice: String,
    pub markPrice: String,
    pub unRealizedProfit: String,
    pub marginAsset: String,
    pub isolatedMargin: String,
    pub positionInitialMargin: String,
    pub initialMargin: String,
    pub maintMargin: String,
    pub updateTime: u64,
}

impl From<RestAccountPosRiskBinanceUM> for PositionData {
    fn from(d: RestAccountPosRiskBinanceUM) -> Self {
        let size = d.positionAmt.parse().unwrap_or_default();
        let avg_price = d.entryPrice.parse().unwrap_or_default();
        let mark_price = d.markPrice.parse().unwrap_or_default();
        let margin = d.positionInitialMargin.parse().unwrap_or_default();
        let leverage = if margin != 0.0 {
            size * avg_price / margin
        } else {
            0.0
        };

        PositionData {
            timestamp: ts_to_micros(d.updateTime),
            inst: binance_inst_to_cli(&d.symbol),
            inst_type: if d.symbol.contains('_') {
                InstrumentType::Futures
            } else {
                InstrumentType::Perpetual
            },
            position_side: match d.positionSide.as_str() {
                "BOTH" => PositionSide::Both,
                "LONG" => PositionSide::Long,
                "SHORT" => PositionSide::Short,
                _ => PositionSide::Unknown,
            },
            size,
            avg_price,
            mark_price,
            margin,
            leverage,
        }
    }
}
