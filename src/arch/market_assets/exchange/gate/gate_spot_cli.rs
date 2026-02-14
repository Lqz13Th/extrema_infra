use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::account_data::OrderAckData,
        api_general::{OrderParams, RequestMethod},
        base_data::{OrderSide, OrderType, TimeInForce},
        exchange::gate::{
            config_assets::{GATE_BASE_URL, GATE_SPOT_ORDERS},
            gate_rest_msg::RestResGate,
            schemas::spot_rest::order::RestOrderGateSpot,
        },
    },
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{GateKey, read_gate_env_key},
    api_utils::normalize_gate_text,
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

impl LobPublicRest for GateSpotCli {}

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

impl LobWebsocket for GateSpotCli {}

impl GateSpotCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
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

        let tif = match order_params.order_type {
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
        });
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
}
