use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::binance::api_utils::binance_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestExchangeInfoBinanceCM {
    pub symbols: Vec<InstrumentInfoBinanceCM>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct InstrumentInfoBinanceCM {
    pub symbol: String,
    pub contractType: String,
    pub status: String,
    pub pricePrecision: i32,
    pub quantityPrecision: i32,
    filters: Vec<Filter>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Deserialize)]
enum Filter {
    PRICE_FILTER(PriceFilter),
    LOT_SIZE(SizeFilter),
    MARKET_LOT_SIZE(SizeFilter),
    #[serde(other)]
    Other,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PriceFilter {
    maxPrice: String,
    minPrice: String,
    tickSize: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct SizeFilter {
    maxQty: String,
    minQty: String,
    stepSize: String,
}

impl From<InstrumentInfoBinanceCM> for InstrumentInfo {
    fn from(d: InstrumentInfoBinanceCM) -> Self {
        let mut tick_size = 0.0;
        let mut min_lmt_size = 0.0;
        let mut max_lmt_size = 0.0;
        let mut min_mkt_size = 0.0;
        let mut max_mkt_size = 0.0;

        let mut lot_size_lmt = 0.0;
        let mut lot_size_mkt = 0.0;

        for f in d.filters.iter() {
            match f {
                Filter::PRICE_FILTER(pf) => {
                    tick_size = pf.tickSize.parse().unwrap_or_default();
                },
                Filter::LOT_SIZE(sf) => {
                    lot_size_lmt = sf.stepSize.parse::<f64>().unwrap_or_default();
                    min_lmt_size = sf.minQty.parse().unwrap_or_default();
                    max_lmt_size = sf.maxQty.parse().unwrap_or_default();
                },
                Filter::MARKET_LOT_SIZE(sf) => {
                    lot_size_mkt = sf.stepSize.parse::<f64>().unwrap_or_default();
                    min_mkt_size = sf.minQty.parse().unwrap_or_default();
                    max_mkt_size = sf.maxQty.parse().unwrap_or_default();
                },
                Filter::Other => {},
            };
        }

        InstrumentInfo {
            inst: binance_inst_to_cli(&d.symbol),
            inst_code: None,
            inst_type: match d.contractType.as_str() {
                "CURRENT_QUARTER" => InstrumentType::Futures,
                "PERPETUAL" => InstrumentType::Perpetual,
                "SPOT" => InstrumentType::Spot,
                _ => InstrumentType::Unknown,
            },
            lot_size: lot_size_lmt.max(lot_size_mkt),
            tick_size,
            min_lmt_size,
            max_lmt_size,
            min_mkt_size,
            max_mkt_size,
            contract_value: None,
            contract_multiplier: None,
            state: match d.status.as_str() {
                "TRADING" => InstrumentStatus::Live,
                _ => InstrumentStatus::Suspend,
            },
        }
    }
}
