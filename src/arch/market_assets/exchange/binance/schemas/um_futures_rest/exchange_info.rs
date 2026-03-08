use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::binance::api_utils::binance_fut_inst_to_cli,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestExchangeInfoBinanceUM {
    pub symbols: Vec<InstrumentInfoBinanceUM>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct InstrumentInfoBinanceUM {
    pub symbol: String,
    pub contractType: String,
    pub status: String,
    pub pricePrecision: i32,
    pub quantityPrecision: i32,
    filters: Vec<Filter>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "filterType")]
enum Filter {
    PRICE_FILTER(PriceFilter),
    LOT_SIZE(SizeFilter),
    MARKET_LOT_SIZE(SizeFilter),
    MIN_NOTIONAL(MinNotionalFilter),
    #[serde(rename = "NOTIONAL")]
    Notional(NotionalFilter),
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

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct MinNotionalFilter {
    #[serde(default)]
    minNotional: Option<String>,
    #[serde(default)]
    notional: Option<String>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct NotionalFilter {
    #[serde(default)]
    minNotional: Option<String>,
    #[serde(default)]
    notional: Option<String>,
}

impl From<InstrumentInfoBinanceUM> for InstrumentInfo {
    fn from(d: InstrumentInfoBinanceUM) -> Self {
        let mut tick_size = 0.0;
        let mut min_lmt_size = 0.0;
        let mut max_lmt_size = 0.0;
        let mut min_mkt_size = 0.0;
        let mut max_mkt_size = 0.0;
        let mut min_notional: f64 = 0.0;

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
                Filter::MIN_NOTIONAL(nf) => {
                    min_notional = min_notional.max(
                        nf.minNotional
                            .as_deref()
                            .or(nf.notional.as_deref())
                            .unwrap_or_default()
                            .parse::<f64>()
                            .unwrap_or_default(),
                    );
                },
                Filter::Notional(nf) => {
                    min_notional = min_notional.max(
                        nf.minNotional
                            .as_deref()
                            .or(nf.notional.as_deref())
                            .unwrap_or_default()
                            .parse::<f64>()
                            .unwrap_or_default(),
                    );
                },
                Filter::Other => {},
            };
        }

        InstrumentInfo {
            inst: binance_fut_inst_to_cli(&d.symbol),
            inst_code: None,
            inst_type: match d.contractType.as_str() {
                "PERPETUAL" => InstrumentType::Perpetual,
                "CURRENT_QUARTER" | "NEXT_QUARTER" | "CURRENT_MONTH" | "NEXT_MONTH" => {
                    InstrumentType::Futures
                },
                "SPOT" => InstrumentType::Spot,
                _ => InstrumentType::Unknown,
            },
            lot_size: lot_size_lmt.max(lot_size_mkt),
            tick_size,
            min_lmt_size,
            max_lmt_size,
            min_mkt_size,
            max_mkt_size,
            min_notional: (min_notional > 0.0).then_some(min_notional),
            contract_value: None,
            contract_multiplier: None,
            state: match d.status.as_str() {
                "TRADING" => InstrumentStatus::Live,
                _ => InstrumentStatus::Suspend,
            },
        }
    }
}
