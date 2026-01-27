use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        base_data::{InstrumentType, OrderSide, OrderStatus, OrderType},
        exchange::okx::api_utils::okx_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::WsAccOrder,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsAccountOrderOkx {
    ordId: String,
    clOrdId: String,
    instId: String,
    instType: String,
    side: String,
    posSide: Option<String>,
    tdMode: String,
    ordType: String,
    state: String,
    px: Option<String>,
    sz: String,
    fillPx: Option<String>,
    fillSz: Option<String>,
    fillPnl: Option<String>,
    fillTime: Option<String>,
    tradeId: Option<String>,
    fee: Option<String>,
    feeCcy: Option<String>,
    uTime: String,
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
                "OPTION" => InstrumentType::Options,
                _ => InstrumentType::Unknown,
            },
            price: self
                .px
                .as_ref()
                .and_then(|p| p.parse::<f64>().ok())
                .unwrap_or(0.0),
            size: self.sz.parse().unwrap_or_default(),
            filled_size: self
                .fillSz
                .as_ref()
                .and_then(|sz| sz.parse::<f64>().ok())
                .unwrap_or(0.0),
            side: match self.side.as_str() {
                "buy" => OrderSide::BUY,
                "sell" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            status: match self.state.as_str() {
                "partially_filled" => OrderStatus::PartiallyFilled,
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
