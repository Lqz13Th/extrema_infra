use serde::Deserialize;

use crate::market_assets::{
    api_general::ts_to_micros,
    cex::okx::api_utils::okx_inst_to_cli,
    market_core::Market,
    base_data::{InstrumentType, OrderSide, OrderStatus, OrderType},
};
use crate::strategy_base::handler::cex_events::WsAccOrder;
use crate::traits::conversion::IntoWsData;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct WsAccountOrderOkx {
    pub ordId: String,
    pub clOrdId: String,
    pub instId: String,
    pub instType: String,
    pub side: String,
    pub posSide: Option<String>,
    pub tdMode: String,
    pub ordType: String,
    pub state: String,
    pub px: Option<String>,
    pub sz: String,
    pub fillPx: Option<String>,
    pub fillSz: Option<String>,
    pub fillPnl: Option<String>,
    pub fillTime: Option<String>,
    pub tradeId: Option<String>,
    pub fee: Option<String>,
    pub feeCcy: Option<String>,
    pub uTime: String,
}

impl IntoWsData for WsAccountOrderOkx {
    type Output = WsAccOrder;

    fn into_ws(self) -> Self::Output {
        WsAccOrder {
            timestamp: ts_to_micros(self.uTime.parse().unwrap_or_default()),
            market: Market::Okx,
            inst: okx_inst_to_cli(&self.instId),
            inst_type: match self.instType.as_str() {
                "SPOT" => InstrumentType::Spot,
                "SWAP" => InstrumentType::Perpetual,
                "OPTION" => InstrumentType::Option,
                _ => InstrumentType::Unknown,
            },
            price: self.px
                .as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0),
            size: self.sz.parse().unwrap_or_default(),
            filled_size: self.fillSz
                .as_ref()
                .and_then(|sz| sz.parse::<f64>().ok())
                .unwrap_or(0.0),
            side: match self.side.as_str() {
                "buy" => OrderSide::BUY,
                "sell" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            status: match self.state.as_str() {
                "live" => OrderStatus::Live,
                "filled" => OrderStatus::Filled,
                "canceled" | "mmp_canceled" => OrderStatus::Canceled,
                _ => OrderStatus::Unknown,
            },
            order_type: match self.ordType.as_str() {
                "market" => OrderType::Market,
                "limit" => OrderType::Limit,
                _ => OrderType::Unknown,
            },
            cli_order_id: if self.clOrdId.is_empty() {
                None
            } else {
                Some(self.clOrdId)
            },
        }
    }
}
