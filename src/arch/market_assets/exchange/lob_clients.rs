#![allow(unused_imports)]

use super::prelude::*;
use crate::arch::{
    market_assets::{
        api_data::{account_data::*, price_data::*, utils_data::*},
        api_general::{CancelOrderParams, OrderParams},
        base_data::InstrumentType,
    },
    strategy_base::command::command_core::WsConnectTarget,
    task_execution::task_ws::{CandleParam, WsChannel},
    traits::market_lob::*,
};
use crate::errors::{InfraError, InfraResult};

/// Unified dispatcher for the built-in limit-order-book exchange clients.
///
/// Every method delegates to the selected concrete client. Operations that
/// client does not support return [`InfraError::Unimplemented`].
#[derive(Clone, Debug)]
#[cfg(feature = "lob_clients")]
pub enum LobClients {
    Hyperliquid(HyperliquidCli),
    BinanceCm(BinanceCmCli),
    BinanceSpot(BinanceSpotCli),
    BinanceUm(BinanceUmCli),
    GateDelivery(GateDeliveryCli),
    GateFutures(GateFuturesCli),
    GateSpot(GateSpotCli),
    GateUni(GateUniCli),
    Okx(OkxCli),
}

#[cfg(feature = "lob_clients")]
impl Default for LobClients {
    fn default() -> Self {
        LobClients::Hyperliquid(HyperliquidCli::default())
    }
}

#[cfg(feature = "lob_clients")]
impl MarketLobApi for LobClients {}

#[cfg(feature = "lob_clients")]
macro_rules! dispatch {
    ($self:ident, $c:ident => $call:expr) => {
        match $self {
            LobClients::Hyperliquid($c) => $call,
            LobClients::BinanceCm($c) => $call,
            LobClients::BinanceSpot($c) => $call,
            LobClients::BinanceUm($c) => $call,
            LobClients::GateDelivery($c) => $call,
            LobClients::GateFutures($c) => $call,
            LobClients::GateSpot($c) => $call,
            LobClients::GateUni($c) => $call,
            LobClients::Okx($c) => $call,
        }
    };
}

#[cfg(feature = "lob_clients")]
impl LobPublicRest for LobClients {
    async fn get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        dispatch!(self, c => c.get_tickers(insts, inst_type).await)
    }

    async fn get_mark_prices(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<MarkPriceData>> {
        dispatch!(self, c => c.get_mark_prices(insts, inst_type).await)
    }

    async fn get_orderbook(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        depth: usize,
    ) -> InfraResult<OrderBookData> {
        dispatch!(self, c => c.get_orderbook(inst, inst_type, depth).await)
    }

    async fn get_candles(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        interval: CandleParam,
        limit: Option<u32>,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
    ) -> InfraResult<Vec<CandleData>> {
        dispatch!(self, c => {
            c.get_candles(inst, inst_type, interval, limit, start_time_us, end_time_us)
                .await
        })
    }

    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        dispatch!(self, c => c.get_instrument_info(inst_type).await)
    }

    async fn get_live_instruments(&self, inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        dispatch!(self, c => c.get_live_instruments(inst_type).await)
    }
}

#[cfg(feature = "lob_clients")]
impl LobPrivateRest for LobClients {
    fn init_api_key(&mut self) {
        dispatch!(self, c => c.init_api_key())
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        dispatch!(self, c => c.place_order(order_params).await)
    }

    async fn place_orders(&self, order_params: Vec<OrderParams>) -> InfraResult<Vec<OrderAckData>> {
        dispatch!(self, c => c.place_orders(order_params).await)
    }

    async fn cancel_order(
        &self,
        inst: &str,
        order_id: Option<&str>,
        cli_order_id: Option<&str>,
    ) -> InfraResult<OrderAckData> {
        dispatch!(self, c => c.cancel_order(inst, order_id, cli_order_id).await)
    }

    async fn cancel_orders(
        &self,
        cancel_params: Vec<CancelOrderParams>,
    ) -> InfraResult<Vec<OrderAckData>> {
        dispatch!(self, c => c.cancel_orders(cancel_params).await)
    }

    async fn get_open_orders(
        &self,
        inst: &str,
        limit: Option<u32>,
    ) -> InfraResult<Vec<OrderDetailData>> {
        dispatch!(self, c => c.get_open_orders(inst, limit).await)
    }

    async fn get_balance(&self, insts: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        dispatch!(self, c => c.get_balance(insts).await)
    }

    async fn get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        dispatch!(self, c => c.get_positions(insts).await)
    }

    async fn get_order_history(
        &self,
        inst: &str,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
        limit: Option<u32>,
    ) -> InfraResult<Vec<OrderDetailData>> {
        dispatch!(self, c => {
            c.get_order_history(inst, start_time_us, end_time_us, limit)
                .await
        })
    }

    async fn get_order(&self, inst: &str, order_id: &str) -> InfraResult<OrderDetailData> {
        dispatch!(self, c => c.get_order(inst, order_id).await)
    }
}

#[cfg(feature = "lob_clients")]
impl LobWebsocket for LobClients {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        dispatch!(self, c => c.get_public_sub_msg(channel, insts).await)
    }

    async fn get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        dispatch!(self, c => c.get_private_sub_msg(channel).await)
    }

    async fn get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        dispatch!(self, c => c.get_public_connect_msg(channel).await)
    }

    async fn get_public_connect_target(&self, channel: &WsChannel) -> InfraResult<WsConnectTarget> {
        dispatch!(self, c => c.get_public_connect_target(channel).await)
    }

    async fn get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        dispatch!(self, c => c.get_private_connect_msg(channel).await)
    }

    async fn get_private_connect_target(
        &self,
        channel: &WsChannel,
    ) -> InfraResult<WsConnectTarget> {
        dispatch!(self, c => c.get_private_connect_target(channel).await)
    }
}

#[cfg(all(test, feature = "lob_clients"))]
mod tests {
    use super::*;
    use crate::arch::task_execution::task_ws::LobParam;

    #[tokio::test]
    async fn unsupported_operations_stay_unimplemented() {
        let gate_uni = LobClients::GateUni(GateUniCli::default());
        let binance_cm = LobClients::BinanceCm(BinanceCmCli::default());

        assert!(matches!(
            gate_uni
                .get_live_instruments(InstrumentType::Perpetual)
                .await,
            Err(InfraError::Unimplemented)
        ));
        assert!(matches!(
            binance_cm.place_orders(Vec::new()).await,
            Err(InfraError::Unimplemented)
        ));
        assert!(matches!(
            binance_cm.cancel_orders(Vec::new()).await,
            Err(InfraError::Unimplemented)
        ));
        assert!(matches!(
            binance_cm.get_open_orders("BTC_USD_PERP", None).await,
            Err(InfraError::Unimplemented)
        ));
        assert!(matches!(
            binance_cm
                .get_order_history("BTC_USD_PERP", None, None, None)
                .await,
            Err(InfraError::Unimplemented)
        ));
    }

    #[tokio::test]
    async fn websocket_messages_come_from_the_selected_client() {
        let channel = WsChannel::Lob(Some(LobParam::Bbo { frequency: None }));
        let insts = vec!["BTC_USDT_PERP".to_string()];
        let direct = BinanceUmCli::default()
            .get_public_sub_msg(&channel, Some(&insts))
            .await
            .unwrap();
        let dispatched = LobClients::BinanceUm(BinanceUmCli::default())
            .get_public_sub_msg(&channel, Some(&insts))
            .await
            .unwrap();

        assert_eq!(dispatched, direct);
        assert_eq!(
            LobClients::Okx(OkxCli::default())
                .get_public_connect_msg(&channel)
                .await
                .unwrap(),
            OkxCli::default()
                .get_public_connect_msg(&channel)
                .await
                .unwrap()
        );
    }
}
