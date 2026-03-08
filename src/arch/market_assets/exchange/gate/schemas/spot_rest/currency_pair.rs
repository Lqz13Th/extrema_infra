use serde::Deserialize;

use crate::arch::market_assets::{
    api_data::utils_data::InstrumentInfo,
    base_data::{InstrumentStatus, InstrumentType},
};

#[derive(Clone, Debug, Deserialize)]
pub struct RestCurrencyPairGateSpot {
    pub id: String,
    #[serde(default)]
    pub precision: u32,
    #[serde(default)]
    pub amount_precision: u32,
    #[serde(default)]
    pub min_base_amount: String,
    #[serde(default)]
    pub min_quote_amount: String,
    #[serde(default)]
    pub max_base_amount: String,
    #[serde(default)]
    pub market_order_max_stock: String,
    #[serde(default)]
    pub trade_status: String,
}

impl From<RestCurrencyPairGateSpot> for InstrumentInfo {
    fn from(d: RestCurrencyPairGateSpot) -> Self {
        let tick_size = 10f64.powi(-(d.precision as i32));
        let lot_size = 10f64.powi(-(d.amount_precision as i32));
        let min_base_amount = d.min_base_amount.parse::<f64>().unwrap_or_default();
        let max_base_amount = d.max_base_amount.parse::<f64>().unwrap_or(f64::MAX);
        let market_order_max_stock = d
            .market_order_max_stock
            .parse::<f64>()
            .unwrap_or(max_base_amount);

        InstrumentInfo {
            inst: d.id,
            inst_code: None,
            inst_type: InstrumentType::Spot,
            lot_size,
            tick_size,
            min_lmt_size: min_base_amount.max(lot_size),
            max_lmt_size: max_base_amount,
            min_mkt_size: min_base_amount.max(lot_size),
            max_mkt_size: market_order_max_stock,
            min_notional: d.min_quote_amount.parse::<f64>().ok().filter(|v| *v > 0.0),
            contract_value: None,
            contract_multiplier: None,
            state: match d.trade_status.as_str() {
                "tradable" => InstrumentStatus::Live,
                "" => InstrumentStatus::Unknown,
                _ => InstrumentStatus::Suspend,
            },
        }
    }
}
