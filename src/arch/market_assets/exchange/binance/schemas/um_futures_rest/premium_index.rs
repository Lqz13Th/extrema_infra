use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::{price_data::MarkPriceData, utils_data::FundingRateData},
    api_general::ts_to_micros,
    base_data::InstrumentType,
    exchange::binance::api_utils::binance_fut_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestPremiumIndexBinanceUM {
    pub symbol: String,
    pub markPrice: String,
    pub indexPrice: String,
    pub estimatedSettlePrice: String,
    pub lastFundingRate: String,
    pub interestRate: String,
    pub nextFundingTime: u64,
    pub time: u64,
}

impl From<RestPremiumIndexBinanceUM> for FundingRateData {
    fn from(d: RestPremiumIndexBinanceUM) -> Self {
        FundingRateData {
            timestamp: ts_to_micros(d.time),
            inst: binance_fut_inst_to_cli(&d.symbol),
            funding_rate: d.lastFundingRate.parse().unwrap_or_default(),
            funding_time: ts_to_micros(d.nextFundingTime),
        }
    }
}

impl From<RestPremiumIndexBinanceUM> for MarkPriceData {
    fn from(d: RestPremiumIndexBinanceUM) -> Self {
        let inst_type = if d.symbol.contains('_') {
            InstrumentType::Futures
        } else {
            InstrumentType::Perpetual
        };

        MarkPriceData {
            timestamp: ts_to_micros(d.time),
            inst: binance_fut_inst_to_cli(&d.symbol),
            inst_type,
            mark_price: d.markPrice.parse().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn converts_premium_index_mark_price() {
        let raw: RestPremiumIndexBinanceUM = serde_json::from_value(json!({
            "symbol": "BTCUSDT",
            "markPrice": "64123.40000000",
            "indexPrice": "64118.20000000",
            "estimatedSettlePrice": "64120.00000000",
            "lastFundingRate": "0.00010000",
            "interestRate": "0.00010000",
            "nextFundingTime": 1756800000000u64,
            "time": 1756799990000u64
        }))
        .unwrap();

        let data = MarkPriceData::from(raw);

        assert_eq!(data.timestamp, 1756799990000000);
        assert_eq!(data.inst, "BTC_USDT_PERP");
        assert_eq!(data.inst_type, InstrumentType::Perpetual);
        assert_eq!(data.mark_price, 64123.4);
    }
}
