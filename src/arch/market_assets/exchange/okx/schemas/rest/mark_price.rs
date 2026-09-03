use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::price_data::MarkPriceData, api_general::ts_to_micros, base_data::InstrumentType,
    exchange::okx::api_utils::okx_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestMarkPriceOkx {
    pub instType: String,
    pub instId: String,
    pub markPx: String,
    pub ts: String,
}

impl From<RestMarkPriceOkx> for MarkPriceData {
    fn from(d: RestMarkPriceOkx) -> Self {
        MarkPriceData {
            timestamp: ts_to_micros(d.ts.parse().unwrap_or_default()),
            inst: okx_inst_to_cli(&d.instId),
            inst_type: match d.instType.as_str() {
                "FUTURES" => InstrumentType::Futures,
                "SWAP" => InstrumentType::Perpetual,
                _ => InstrumentType::Unknown,
            },
            mark_price: d.markPx.parse().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn converts_swap_mark_price() {
        let raw: RestMarkPriceOkx = serde_json::from_value(json!({
            "instType": "SWAP",
            "instId": "BTC-USDT-SWAP",
            "markPx": "64123.4",
            "ts": "1756800000000"
        }))
        .unwrap();

        let data = MarkPriceData::from(raw);

        assert_eq!(data.timestamp, 1756800000000000);
        assert_eq!(data.inst, "BTC_USDT_PERP");
        assert_eq!(data.inst_type, InstrumentType::Perpetual);
        assert_eq!(data.mark_price, 64123.4);
    }
}
