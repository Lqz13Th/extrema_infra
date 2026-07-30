use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::account_data::PositionData,
    api_general::ts_to_micros,
    base_data::{InstrumentType, PositionSide},
    exchange::binance::api_utils::binance_fut_inst_to_cli,
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
        let size = d.positionAmt.parse::<f64>().unwrap_or_default();
        let avg_price = d.entryPrice.parse::<f64>().unwrap_or_default();
        let mark_price = d.markPrice.parse::<f64>().unwrap_or_default();
        let margin = d.positionInitialMargin.parse::<f64>().unwrap_or_default();
        let leverage = if margin != 0.0 {
            (size * avg_price).abs() / margin.abs()
        } else {
            0.0
        };

        PositionData {
            timestamp: ts_to_micros(d.updateTime),
            inst: binance_fut_inst_to_cli(&d.symbol),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn position(position_amount: &str, margin: &str) -> RestAccountPosRiskBinanceUM {
        RestAccountPosRiskBinanceUM {
            symbol: "BTCUSDT".into(),
            positionSide: "BOTH".into(),
            positionAmt: position_amount.into(),
            entryPrice: "50000".into(),
            markPrice: "51000".into(),
            unRealizedProfit: "0".into(),
            marginAsset: "USDT".into(),
            isolatedMargin: "0".into(),
            positionInitialMargin: margin.into(),
            initialMargin: margin.into(),
            maintMargin: "0".into(),
            updateTime: 1_700_000_000_000,
        }
    }

    #[test]
    fn derives_non_negative_leverage_from_absolute_notional() {
        let long = PositionData::from(position("0.2", "1000"));
        let short = PositionData::from(position("-0.2", "1000"));

        assert_eq!(long.leverage, 10.0);
        assert_eq!(short.size, -0.2);
        assert_eq!(short.leverage, 10.0);
    }

    #[test]
    fn derives_zero_leverage_when_margin_is_zero() {
        let position = PositionData::from(position("-0.2", "0"));

        assert_eq!(position.leverage, 0.0);
    }
}
