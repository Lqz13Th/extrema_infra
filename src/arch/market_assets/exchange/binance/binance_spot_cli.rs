use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{account_data::*, price_data::*, utils_data::*},
        api_general::*,
        base_data::*,
        exchange::binance::binance_rest_msg::RestResBinance,
    },
    task_execution::task_ws::WsChannel,
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{BinanceKey, read_binance_env_key},
    api_utils::*,
    config_assets::*,
    schemas::{
        spot_rest::{
            account_balance::RestAccountInfoBinanceSpot,
            exchange_info::RestExchangeInfoBinanceSpot, order_history::RestOrderHistoryBinanceSpot,
            ticker::RestTickerBinanceSpot, trade_order::RestOrderAckBinanceSpot,
        },
        wallet_rest::{
            all_coins_info::RestCapitalConfigCoinBinance,
            deposit_address::RestDepositAddressBinance,
            deposit_address_list::RestDepositAddressListBinance,
            deposit_history::RestDepositHistoryBinance,
            sub_account_transfer::RestSubAccountUniversalTransferBinance,
            sub_account_transfer_history::RestSubAccountTransferHistoryBinance,
            transfer::RestUserUniversalTransferBinance,
            transfer_history::RestTransferHistoryBinance, withdraw::RestWithdrawBinance,
            withdraw_address_list::RestWithdrawAddressBinance,
            withdraw_history::RestWithdrawHistoryBinance,
        },
    },
};

#[derive(Clone, Debug)]
pub struct BinanceSpotCli {
    pub client: Arc<Client>,
    pub api_key: Option<BinanceKey>,
}

impl Default for BinanceSpotCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for BinanceSpotCli {}

impl LobPublicRest for BinanceSpotCli {
    async fn get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        self._get_tickers(insts, inst_type).await
    }

    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        self._get_instrument_info(inst_type).await
    }
}

impl LobPrivateRest for BinanceSpotCli {
    fn init_api_key(&mut self) {
        match read_binance_env_key() {
            Ok(binance_key) => {
                self.api_key = Some(binance_key);
            },
            Err(e) => {
                error!("Failed to read BINANCE env key: {:?}", e);
            },
        };
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        self._place_order(order_params).await
    }

    async fn cancel_order(
        &self,
        inst: &str,
        order_id: Option<&str>,
        cli_order_id: Option<&str>,
    ) -> InfraResult<OrderAckData> {
        self._cancel_order(inst, order_id, cli_order_id).await
    }

    async fn get_open_orders(
        &self,
        inst: &str,
        limit: Option<u32>,
    ) -> InfraResult<Vec<OrderDetailData>> {
        self._get_open_orders(inst, limit).await
    }

    async fn get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
    }

    async fn get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<&str>,
    ) -> InfraResult<Vec<OrderDetailData>> {
        self._get_order_history(inst, start_time, end_time, limit, order_id)
            .await
    }
}

impl LobWebsocket for BinanceSpotCli {
    async fn get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_private_sub_msg(channel)
    }

    async fn get_private_connect_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        Ok(BINANCE_SPOT_WS_API.into())
    }
}

impl BinanceSpotCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    pub async fn user_universal_transfer(
        &self,
        req: BinanceUniversalTransferReq,
    ) -> InfraResult<RestUserUniversalTransferBinance> {
        let res: RestResBinance<RestUserUniversalTransferBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                Some(&req.to_query_string()),
                BINANCE_SPOT_BASE_URL,
                BINANCE_USER_UNIVERSAL_TRANSFER,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No transfer response data returned".into(),
            ))?;

        Ok(data)
    }

    pub async fn sub_account_universal_transfer(
        &self,
        req: BinanceSubAccountUniversalTransferReq,
    ) -> InfraResult<RestSubAccountUniversalTransferBinance> {
        let query = req.to_query_string()?;
        let res: RestResBinance<RestSubAccountUniversalTransferBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                Some(&query),
                BINANCE_SPOT_BASE_URL,
                BINANCE_SUB_ACCOUNT_UNIVERSAL_TRANSFER,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No sub-account transfer response data returned".into(),
            ))?;

        Ok(data)
    }

    pub async fn transfer_spot_to_um(
        &self,
        asset: &str,
        amount: &str,
        recv_window: Option<u64>,
    ) -> InfraResult<RestUserUniversalTransferBinance> {
        self.user_universal_transfer(BinanceUniversalTransferReq {
            transfer_type: BinanceUniversalTransferType::MainUmFuture,
            asset: asset.to_string(),
            amount: amount.to_string(),
            from_symbol: None,
            to_symbol: None,
            recv_window,
        })
        .await
    }

    pub async fn transfer_um_to_spot(
        &self,
        asset: &str,
        amount: &str,
        recv_window: Option<u64>,
    ) -> InfraResult<RestUserUniversalTransferBinance> {
        self.user_universal_transfer(BinanceUniversalTransferReq {
            transfer_type: BinanceUniversalTransferType::UmFutureMain,
            asset: asset.to_string(),
            amount: amount.to_string(),
            from_symbol: None,
            to_symbol: None,
            recv_window,
        })
        .await
    }

    pub async fn get_withdraw_address_list(
        &self,
        coin: &str,
        network: Option<&str>,
    ) -> InfraResult<Vec<RestWithdrawAddressBinance>> {
        let mut query_parts = vec![format!("coin={}", coin.to_uppercase())];
        if let Some(network) = network {
            query_parts.push(format!("network={network}"));
        }

        let query = query_parts.join("&");
        let res: RestResBinance<RestWithdrawAddressBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query),
                BINANCE_SPOT_BASE_URL,
                BINANCE_WITHDRAW_ADDRESS_LIST,
            )
            .await?;

        res.into_vec()
    }

    pub async fn get_universal_transfer_history(
        &self,
        req: BinanceUniversalTransferHistoryReq,
    ) -> InfraResult<RestTransferHistoryBinance> {
        let res: RestResBinance<RestTransferHistoryBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&req.to_query_string()),
                BINANCE_SPOT_BASE_URL,
                BINANCE_USER_UNIVERSAL_TRANSFER,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No transfer history data returned".into(),
            ))?;

        Ok(data)
    }

    pub async fn get_sub_account_universal_transfer_history(
        &self,
        req: BinanceSubAccountUniversalTransferHistoryReq,
    ) -> InfraResult<RestSubAccountTransferHistoryBinance> {
        let query = req.to_query_string()?;
        let res: RestResBinance<RestSubAccountTransferHistoryBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                query.as_deref(),
                BINANCE_SPOT_BASE_URL,
                BINANCE_SUB_ACCOUNT_UNIVERSAL_TRANSFER,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No sub-account transfer history data returned".into(),
            ))?;

        Ok(data)
    }

    pub async fn withdraw(&self, req: BinanceWithdrawReq) -> InfraResult<RestWithdrawBinance> {
        let res: RestResBinance<RestWithdrawBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                Some(&req.to_query_string()),
                BINANCE_SPOT_BASE_URL,
                BINANCE_WITHDRAW_APPLY,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No withdraw response data returned".into(),
            ))?;

        Ok(data)
    }

    pub async fn get_withdraw_history(
        &self,
        req: BinanceWithdrawHistoryReq,
    ) -> InfraResult<Vec<RestWithdrawHistoryBinance>> {
        let query = req.to_query_string();

        let res: RestResBinance<RestWithdrawHistoryBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                query.as_deref(),
                BINANCE_SPOT_BASE_URL,
                BINANCE_WITHDRAW_HISTORY,
            )
            .await?;

        res.into_vec()
    }

    pub async fn get_all_coins_info(
        &self,
        recv_window: Option<u64>,
    ) -> InfraResult<Vec<RestCapitalConfigCoinBinance>> {
        let query = recv_window.map(|window| format!("recvWindow={window}"));

        let res: RestResBinance<RestCapitalConfigCoinBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                query.as_deref(),
                BINANCE_SPOT_BASE_URL,
                BINANCE_CAPITAL_CONFIG_GETALL,
            )
            .await?;

        res.into_vec()
    }

    pub async fn get_deposit_address(
        &self,
        coin: &str,
        network: Option<&str>,
        amount: Option<&str>,
        recv_window: Option<u64>,
    ) -> InfraResult<RestDepositAddressBinance> {
        let mut query_parts = vec![format!("coin={}", coin.to_uppercase())];

        if let Some(network) = network {
            query_parts.push(format!("network={network}"));
        }
        if let Some(amount) = amount {
            query_parts.push(format!("amount={amount}"));
        }
        if let Some(window) = recv_window {
            query_parts.push(format!("recvWindow={window}"));
        }

        let query = query_parts.join("&");
        let res: RestResBinance<RestDepositAddressBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query),
                BINANCE_SPOT_BASE_URL,
                BINANCE_DEPOSIT_ADDRESS,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No deposit address data returned".into(),
            ))?;

        Ok(data)
    }

    pub async fn get_deposit_address_list(
        &self,
        coin: &str,
        network: Option<&str>,
    ) -> InfraResult<Vec<RestDepositAddressListBinance>> {
        let mut query_parts = vec![format!("coin={}", coin.to_uppercase())];
        if let Some(network) = network {
            query_parts.push(format!("network={network}"));
        }

        let query = query_parts.join("&");
        let res: RestResBinance<RestDepositAddressListBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query),
                BINANCE_SPOT_BASE_URL,
                BINANCE_DEPOSIT_ADDRESS_LIST,
            )
            .await?;

        res.into_vec()
    }

    pub async fn get_deposit_history(
        &self,
        req: BinanceDepositHistoryReq,
    ) -> InfraResult<Vec<RestDepositHistoryBinance>> {
        let query = req.to_query_string();

        let res: RestResBinance<RestDepositHistoryBinance> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                query.as_deref(),
                BINANCE_SPOT_BASE_URL,
                BINANCE_DEPOSIT_HISTORY,
            )
            .await?;

        res.into_vec()
    }

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        _inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        let url = format!("{}{}", BINANCE_SPOT_BASE_URL, BINANCE_SPOT_TICKERS);
        let response = self.client.get(url).send().await?;
        let res: RestResBinance<RestTickerBinanceSpot> =
            parse_json_response("BinanceSpot tickers", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .filter(|t| match insts {
                Some(list) => list
                    .iter()
                    .any(|inst| cli_spot_to_binance_spot(inst) == t.symbol), // BTCUSDT
                None => true,
            })
            .map(TickerData::from)
            .collect();

        Ok(data)
    }

    async fn _get_instrument_info(
        &self,
        _inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        let url = [BINANCE_SPOT_BASE_URL, BINANCE_SPOT_EXCHANGE_INFO].concat();

        let response = self.client.get(&url).send().await?;
        let res: RestResBinance<RestExchangeInfoBinanceSpot> =
            parse_json_response("BinanceSpot instrument_info", response).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No exchange info data returned".into(),
            ))?
            .symbols
            .into_iter()
            .map(InstrumentInfo::from)
            .collect();

        Ok(data)
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let (quantity_param, quantity) =
            if let Some(quote_order_qty) = order_params.extra.get("quoteOrderQty") {
                ("quoteOrderQty", quote_order_qty)
            } else {
                ("quantity", &order_params.size)
            };
        let mut query_string = format!(
            "symbol={}&side={}&type={}&{}={}&newOrderRespType=RESULT",
            order_params.inst.to_uppercase(),
            match order_params.side {
                OrderSide::BUY => "BUY",
                OrderSide::SELL => "SELL",
                _ => "BUY",
            },
            match order_params.order_type {
                OrderType::Limit => "LIMIT",
                OrderType::Market => "MARKET",
                OrderType::PostOnly => "LIMIT_MAKER",
                OrderType::Fok => "FOK",
                OrderType::Ioc => "IOC",
                OrderType::Unknown => "MARKET",
            },
            quantity_param,
            quantity,
        );

        if let Some(price) = &order_params.price {
            query_string.push_str(&format!("&price={}", price));
        }

        if let Some(tif) = &order_params.time_in_force {
            let tif_str = match tif {
                TimeInForce::GTC => "GTC",
                TimeInForce::IOC => "IOC",
                TimeInForce::FOK => "FOK",
                TimeInForce::GTD => "GTD",
                TimeInForce::Unknown => "GTC",
            };
            query_string.push_str(&format!("&timeInForce={}", tif_str));
        }

        if let Some(cid) = &order_params.client_order_id {
            query_string.push_str(&format!("&newClientOrderId={}", cid));
        }

        for (k, v) in &order_params.extra {
            if k == "quoteOrderQty" {
                continue;
            }
            query_string.push_str(&format!("&{}={}", k, v));
        }

        let res: RestResBinance<RestOrderAckBinanceSpot> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                Some(&query_string),
                BINANCE_SPOT_BASE_URL,
                BINANCE_SPOT_PLACE_ORDER,
            )
            .await?;

        let data: OrderAckData = res
            .into_vec()?
            .into_iter()
            .map(OrderAckData::from)
            .next()
            .ok_or(InfraError::ApiCliError("No order ack data returned".into()))?;

        Ok(data)
    }

    async fn _cancel_order(
        &self,
        inst: &str,
        order_id: Option<&str>,
        cli_order_id: Option<&str>,
    ) -> InfraResult<OrderAckData> {
        if order_id.is_none() && cli_order_id.is_none() {
            return Err(InfraError::ApiCliError(
                "Binance Spot cancel_order requires order_id or cli_order_id".into(),
            ));
        }

        let mut query_string = format!("symbol={}", cli_spot_to_binance_spot(inst));

        if let Some(oid) = order_id {
            query_string.push_str(&format!("&orderId={}", oid));
        }

        if let Some(cid) = cli_order_id {
            query_string.push_str(&format!("&origClientOrderId={}", cid));
        }

        let res: RestResBinance<RestOrderAckBinanceSpot> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Delete,
                Some(&query_string),
                BINANCE_SPOT_BASE_URL,
                BINANCE_SPOT_CANCEL_ORDER,
            )
            .await?;

        let data: OrderAckData = res
            .into_vec()?
            .into_iter()
            .map(OrderAckData::from)
            .next()
            .ok_or(InfraError::ApiCliError(
                "No cancel ack data returned".into(),
            ))?;

        Ok(data)
    }

    async fn _get_open_orders(
        &self,
        inst: &str,
        limit: Option<u32>,
    ) -> InfraResult<Vec<OrderDetailData>> {
        if limit == Some(0) {
            return Ok(Vec::new());
        }

        let query_string = format!("symbol={}", cli_spot_to_binance_spot(inst));
        let res: RestResBinance<RestOrderHistoryBinanceSpot> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query_string),
                BINANCE_SPOT_BASE_URL,
                BINANCE_SPOT_OPEN_ORDERS,
            )
            .await?;

        let mut data: Vec<OrderDetailData> = res
            .into_vec()?
            .into_iter()
            .map(OrderDetailData::from)
            .collect();
        if let Some(limit) = limit {
            data.truncate(limit as usize);
        }

        Ok(data)
    }

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let res: RestResBinance<RestAccountInfoBinanceSpot> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                None,
                BINANCE_SPOT_BASE_URL,
                BINANCE_SPOT_ACCOUNT_INFO,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No spot account info data returned".into(),
            ))?
            .balances
            .into_iter()
            .filter(|b| match assets {
                Some(list) if !list.is_empty() => list.contains(&b.asset),
                _ => true,
            })
            .map(BalanceData::from)
            .collect();

        Ok(data)
    }

    async fn _get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<&str>,
    ) -> InfraResult<Vec<OrderDetailData>> {
        let mut query_string = format!("symbol={}", cli_spot_to_binance_spot(inst));
        let endpoint = if let Some(order_id) = order_id {
            query_string.push_str(&format!("&orderId={order_id}"));
            BINANCE_SPOT_PLACE_ORDER
        } else {
            if let Some(start_time) = start_time {
                query_string.push_str(&format!("&startTime={start_time}"));
            }
            if let Some(end_time) = end_time {
                query_string.push_str(&format!("&endTime={end_time}"));
            }
            if let Some(limit) = limit {
                query_string.push_str(&format!("&limit={limit}"));
            }
            BINANCE_SPOT_ALL_ORDERS
        };

        let res: RestResBinance<RestOrderHistoryBinanceSpot> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query_string),
                BINANCE_SPOT_BASE_URL,
                endpoint,
            )
            .await?;

        let data: Vec<OrderDetailData> = res
            .into_vec()?
            .into_iter()
            .map(OrderDetailData::from)
            .collect();

        Ok(data)
    }

    fn _get_private_sub_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let recv_window = 5000_u64;
        let query_string = format!("apiKey={}&recvWindow={}", api_key.api_key, recv_window);
        let signature = api_key.ws_sign(&query_string)?;

        let msg = json!({
            "id": 1,
            "method": "userDataStream.subscribe.signature",
            "params": {
                "apiKey": &api_key.api_key,
                "timestamp": signature.timestamp,
                "recvWindow": recv_window,
                "signature": signature.signature,
            }
        });

        Ok(msg.to_string())
    }
}
