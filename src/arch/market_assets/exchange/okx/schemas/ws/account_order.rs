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
    accFillSz: Option<String>,
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
            size: self.sz.parse::<f64>().unwrap_or_default().abs(),
            filled_size: self
                .accFillSz
                .as_ref()
                .and_then(|sz| sz.parse::<f64>().ok())
                .unwrap_or(0.0)
                .abs(),
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
            order_id: (!self.ordId.is_empty()).then_some(self.ordId),
            cli_order_id: (!self.clOrdId.is_empty()).then_some(self.clOrdId),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::arch::traits::conversion::IntoWsData;

    use super::*;

    #[test]
    fn into_ws_preserves_exchange_and_client_order_ids() {
        let raw: WsAccountOrderOkx = serde_json::from_value(json!({
            "ordId": "2234567890",
            "clOrdId": "okx-client-id",
            "instId": "GUN-USDT-SWAP",
            "instType": "SWAP",
            "side": "buy",
            "posSide": "net",
            "tdMode": "cross",
            "ordType": "market",
            "state": "filled",
            "px": null,
            "sz": "4350",
            "fillPx": "0.005857",
            "fillSz": "4350",
            "fillPnl": null,
            "fillTime": "1781905826733",
            "tradeId": "1",
            "fee": "0",
            "feeCcy": "USDT",
            "uTime": "1781905826733"
        }))
        .unwrap();

        let ws = raw.into_ws();

        assert_eq!(ws.order_id.as_deref(), Some("2234567890"));
        assert_eq!(ws.cli_order_id.as_deref(), Some("okx-client-id"));
    }

    fn order_json(
        state: &str,
        sz: &str,
        fill_sz: &str,
        acc_fill_sz: Option<&str>,
    ) -> WsAccountOrderOkx {
        let mut value = json!({
            "ordId": "3909420352422744064",
            "clOrdId": "",
            "instId": "BONK-USDT-SWAP",
            "instType": "SWAP",
            "side": "sell",
            "posSide": "net",
            "tdMode": "cross",
            "ordType": "market",
            "state": state,
            "px": null,
            "sz": sz,
            "fillPx": "0.00000283",
            "fillSz": fill_sz,
            "fillPnl": null,
            "fillTime": "1789012205691",
            "tradeId": "2",
            "fee": "0",
            "feeCcy": "USDT",
            "uTime": "1789012205691"
        });
        if let Some(acc) = acc_fill_sz {
            value["accFillSz"] = json!(acc);
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn filled_size_is_the_accumulated_fill_not_the_last_one() {
        // Two fills: 1 then 99. The second push carries fillSz=99, accFillSz=100.
        let ws = order_json("filled", "100", "99", Some("100")).into_ws();
        assert_eq!(ws.status, OrderStatus::Filled);
        assert_eq!(ws.size, 100.0);
        assert_eq!(ws.filled_size, 100.0);

        let partial = order_json("partially_filled", "100", "1", Some("1")).into_ws();
        assert_eq!(partial.filled_size, 1.0);
    }

    #[test]
    fn the_last_fill_alone_is_never_used_as_filled_size() {
        let ws = order_json("filled", "100", "99", None).into_ws();
        assert_eq!(ws.filled_size, 0.0);
    }
}
