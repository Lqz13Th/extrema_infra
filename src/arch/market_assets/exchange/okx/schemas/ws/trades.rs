use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        exchange::okx::api_utils::okx_inst_to_cli,
        base_data::OrderSide,
        market_core::Market,
    },
    strategy_base::handler::cex_events::WsTrade,
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsTradesOkx {
    instId: String,
    tradeId: String,
    px: String,
    sz: String,
    side: String,
    ts: String,
}

impl IntoWsData for WsTradesOkx {
    type Output = WsTrade;

    fn into_ws(self) -> Self::Output {
        WsTrade {
            timestamp: ts_to_micros(self.ts.parse().unwrap_or_default()),
            market: Market::Okx,
            inst: okx_inst_to_cli(&self.instId),
            price: self.px.parse().unwrap_or_default(),
            size: self.sz.parse().unwrap_or_default(),
            side: match self.side.as_str() {
                "buy" => OrderSide::BUY,
                "sell" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            trade_id: self.tradeId.parse().unwrap_or_default(),
        }
    }
}
