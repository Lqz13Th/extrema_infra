use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros, base_data::OrderSide,
        exchange::hyperliquid::api_utils::hyperliquid_inst_to_cli, market_core::Market,
    },
    strategy_base::handler::lob_events::WsTrade,
    traits::conversion::IntoWsData,
};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsTradeHyperliquid {
    coin: String,
    side: String,
    px: String,
    sz: String,
    time: u64,
    tid: u64,
}

impl IntoWsData for WsTradeHyperliquid {
    type Output = WsTrade;

    fn into_ws(self) -> Self::Output {
        WsTrade {
            timestamp: ts_to_micros(self.time),
            market: Market::HyperLiquid,
            inst: hyperliquid_inst_to_cli(&self.coin),
            price: self.px.parse().unwrap_or_default(),
            size: self.sz.parse().unwrap_or_default(),
            side: match self.side.as_str() {
                "B" => OrderSide::BUY,
                "A" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            trade_id: self.tid,
        }
    }
}
