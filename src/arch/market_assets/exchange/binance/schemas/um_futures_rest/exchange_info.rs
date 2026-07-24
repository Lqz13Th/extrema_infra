use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    api_general::get_mills_timestamp,
    base_data::{InstrumentStatus, InstrumentType},
    exchange::binance::api_utils::binance_fut_inst_to_cli,
};

const BINANCE_PERP_FAR_FUTURE_DELIVERY_MS: u64 = 3_786_912_000_000; // 2090-01-01T00:00:00Z

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
    pub deliveryDate: u64,
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
        let now_ms = get_mills_timestamp();
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
                "PERPETUAL" | "TRADIFI_PERPETUAL" => InstrumentType::Perpetual,
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
            max_leverage: None,
            min_notional: (min_notional > 0.0).then_some(min_notional),
            contract_value: None,
            contract_multiplier: None,
            state: binance_status_to_instrument_status(
                &d.status,
                d.contractType.as_str(),
                d.deliveryDate,
                now_ms,
            ),
        }
    }
}

fn binance_status_to_instrument_status(
    status: &str,
    contract_type: &str,
    delivery_date_ms: u64,
    now_ms: u64,
) -> InstrumentStatus {
    match status {
        "SETTLING" => InstrumentStatus::Delisting,
        "TRADING"
            if contract_type == "PERPETUAL"
                && delivery_date_ms > now_ms
                && delivery_date_ms < BINANCE_PERP_FAR_FUTURE_DELIVERY_MS =>
        {
            InstrumentStatus::Delisting
        },
        "TRADING" => InstrumentStatus::Live,
        "PENDING_TRADING" | "PRE_DELIVERING" => InstrumentStatus::PreOpen,
        "DELIVERING" | "PRE_SETTLE" => InstrumentStatus::Delisting,
        "CLOSE" => InstrumentStatus::Closed,
        _ => InstrumentStatus::Suspend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size_filter(min_qty: &str, max_qty: &str, step_size: &str) -> SizeFilter {
        SizeFilter {
            minQty: min_qty.to_string(),
            maxQty: max_qty.to_string(),
            stepSize: step_size.to_string(),
        }
    }

    fn qcom_tradifi_perpetual() -> InstrumentInfoBinanceUM {
        InstrumentInfoBinanceUM {
            symbol: "QCOMUSDT".to_string(),
            contractType: "TRADIFI_PERPETUAL".to_string(),
            status: "TRADING".to_string(),
            deliveryDate: 4_133_404_800_000,
            pricePrecision: 5,
            quantityPrecision: 2,
            filters: vec![
                Filter::PRICE_FILTER(PriceFilter {
                    minPrice: "0.01000".to_string(),
                    maxPrice: "20000".to_string(),
                    tickSize: "0.01000".to_string(),
                }),
                Filter::LOT_SIZE(size_filter("0.01", "20000", "0.01")),
                Filter::MARKET_LOT_SIZE(size_filter("0.01", "2000", "0.01")),
                Filter::MIN_NOTIONAL(MinNotionalFilter {
                    minNotional: None,
                    notional: Some("5".to_string()),
                }),
            ],
        }
    }

    #[test]
    fn tradifi_perpetual_maps_to_perpetual_instrument() {
        let info = InstrumentInfo::from(qcom_tradifi_perpetual());

        assert_eq!(info.inst, "QCOM_USDT_PERP");
        assert_eq!(info.inst_type, InstrumentType::Perpetual);
        assert_eq!(info.state, InstrumentStatus::Live);
        assert_eq!(info.lot_size, 0.01);
        assert_eq!(info.min_mkt_size, 0.01);
        assert_eq!(info.max_mkt_size, 2000.0);
        assert_eq!(info.min_notional, Some(5.0));
    }
}
