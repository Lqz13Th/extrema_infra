use reqwest::Client;
use serde_json::json;
use simd_json::from_slice;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{
            account_data::OrderAckData, price_data::TickerData, utils_data::InstrumentInfo,
        },
        api_general::{OrderParams, RequestMethod, get_seconds_timestamp},
        base_data::{InstrumentType, OrderSide, OrderType, SUBSCRIBE_LOWER, TimeInForce},
        exchange::gate::{
            config_assets::{
                GATE_BASE_URL, GATE_SPOT_CURRENCY_PAIRS, GATE_SPOT_ORDERS, GATE_SPOT_TICKERS,
                GATE_WITHDRAWALS, GATE_WS_BASE_URL, GATE_WS_SPOT_ORDERS_V2,
            },
            gate_rest_msg::RestResGate,
            schemas::{
                spot_rest::{
                    currency_pair::RestCurrencyPairGateSpot, order::RestOrderGateSpot,
                    ticker::RestTickerGateSpot,
                },
                wallet_rest::withdraw::RestWithdrawGate,
            },
        },
    },
    task_execution::task_ws::WsChannel,
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{GateKey, read_gate_env_key},
    api_utils::*,
};

#[derive(Clone, Debug)]
pub struct GateSpotCli {
    pub client: Arc<Client>,
    pub api_key: Option<GateKey>,
}

impl Default for GateSpotCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for GateSpotCli {}

impl LobPublicRest for GateSpotCli {
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

    async fn get_live_instruments(&self, inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        self._get_live_instruments(inst_type).await
    }
}

impl LobPrivateRest for GateSpotCli {
    fn init_api_key(&mut self) {
        match read_gate_env_key() {
            Ok(gate_key) => {
                self.api_key = Some(gate_key);
            },
            Err(e) => {
                error!("Failed to read GATE env key: {:?}", e);
            },
        };
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        self._place_order(order_params).await
    }
}

impl LobWebsocket for GateSpotCli {
    async fn get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_private_sub_msg(channel)
    }

    async fn get_private_connect_msg(&self, _channel: &WsChannel) -> InfraResult<String> {
        Ok(GATE_WS_BASE_URL.into())
    }
}

impl GateSpotCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    pub async fn withdraw(&self, req: GateWithdrawReq) -> InfraResult<RestWithdrawGate> {
        let res: RestResGate<RestWithdrawGate> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                None,
                Some(&req.to_body_string()),
                GATE_BASE_URL,
                GATE_WITHDRAWALS,
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

    fn ws_subscribe_private(&self, channel: &str) -> InfraResult<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?;

        let timestamp = get_seconds_timestamp();
        let auth = api_key.ws_auth(channel, SUBSCRIBE_LOWER, timestamp)?;
        let payload = vec!["!all".to_string()];

        let msg = json!({
            "time": timestamp,
            "channel": channel,
            "event": SUBSCRIBE_LOWER,
            "payload": payload,
            "auth": auth,
        });

        Ok(msg.to_string())
    }

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        _inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        let url = [GATE_BASE_URL, GATE_SPOT_TICKERS].concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestTickerGateSpot> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .filter(|t| match insts {
                Some(list) => list.contains(&t.currency_pair), // BTC_USDT
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
        let url = [GATE_BASE_URL, GATE_SPOT_CURRENCY_PAIRS].concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestCurrencyPairGateSpot> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(InstrumentInfo::from)
            .collect();

        Ok(data)
    }

    async fn _get_live_instruments(&self, _inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        let url = [GATE_BASE_URL, GATE_SPOT_CURRENCY_PAIRS].concat();

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestCurrencyPairGateSpot> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .filter(|p| p.trade_status.as_str() == "tradable")
            .map(|p| p.id)
            .collect();

        Ok(data)
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let mut body = json!({
            "currency_pair": order_params.inst,
            "side": match order_params.side {
                OrderSide::BUY => "buy",
                OrderSide::SELL => "sell",
                _ => "buy",
            },
            "amount": order_params.size,
            "type": match order_params.order_type {
                OrderType::Market => "market",
                _ => "limit",
            },
        });

        if matches!(order_params.order_type, OrderType::Market) {
            if let Some(price) = order_params.price {
                body["price"] = json!(price);
            }
        } else {
            let price = order_params.price.ok_or(InfraError::ApiCliError(
                "Price required for limit order".into(),
            ))?;
            body["price"] = json!(price);
        }

        let mut extra = order_params.extra;
        if let Some(account) = extra.remove("account") {
            body["account"] = json!(account);
        } else {
            body["account"] = json!("spot");
        }

        let tif = if matches!(order_params.order_type, OrderType::Market) {
            match order_params.time_in_force.as_ref() {
                Some(TimeInForce::IOC) => Some("ioc"),
                Some(TimeInForce::FOK) => Some("fok"),
                _ => Some("ioc"),
            }
        } else {
            match order_params.order_type {
                OrderType::PostOnly => Some("poc"),
                OrderType::Fok => Some("fok"),
                OrderType::Ioc => Some("ioc"),
                _ => None,
            }
            .or_else(|| {
                order_params.time_in_force.as_ref().map(|t| match t {
                    TimeInForce::GTC => "gtc",
                    TimeInForce::IOC => "ioc",
                    TimeInForce::FOK => "fok",
                    TimeInForce::GTD => "gtd",
                    TimeInForce::Unknown => "gtc",
                })
            })
        };
        if let Some(tif_val) = tif {
            body["time_in_force"] = json!(tif_val);
        }

        if let Some(cl_id) = order_params.client_order_id {
            body["text"] = json!(normalize_gate_text(&cl_id));
        }

        for (k, v) in extra {
            body[k] = json!(v);
        }

        let res: RestResGate<RestOrderGateSpot> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                None,
                Some(&body.to_string()),
                GATE_BASE_URL,
                GATE_SPOT_ORDERS,
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

    fn _get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        let topic = match channel {
            WsChannel::AccountOrders => GATE_WS_SPOT_ORDERS_V2,
            _ => return Err(InfraError::Unimplemented),
        };
        self.ws_subscribe_private(topic)
    }
}
