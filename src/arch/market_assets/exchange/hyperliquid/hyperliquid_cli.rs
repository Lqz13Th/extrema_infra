use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::account_data::OrderAckData,
        api_general::{OrderParams, get_mills_timestamp},
        base_data::{OrderSide, OrderType},
    },
    traits::market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{HyperliquidKey, HyperliquidSignedRequest, read_hyperliquid_env_key},
    config_assets::*,
    hyperliquid_rest_msg::RestResHyperliquid,
    schemas::rest::trade_order::RestOrderResponse,
};

#[derive(Clone, Debug)]
pub struct HyperliquidCli {
    pub client: Arc<Client>,
    pub api_key: Option<HyperliquidKey>,
    pub is_testnet: bool,
}

impl Default for HyperliquidCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for HyperliquidCli {}

impl LobPublicRest for HyperliquidCli {}

impl LobPrivateRest for HyperliquidCli {
    fn init_api_key(&mut self) {
        match read_hyperliquid_env_key() {
            Ok(key) => {
                self.api_key = Some(key);
            },
            Err(e) => {
                error!("Failed to read HYPERLIQUID env key: {:?}", e);
            },
        };
    }

    async fn place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        self._place_order(order_params).await
    }
}

impl LobWebsocket for HyperliquidCli {}

impl HyperliquidCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
            is_testnet: false,
        }
    }

    pub fn with_testnet(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
            is_testnet: true,
        }
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let asset = order_params
            .extra
            .get("asset")
            .or_else(|| order_params.extra.get("a"))
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or(InfraError::ApiCliError(
                "Missing Hyperliquid asset index in OrderParams.extra".into(),
            ))?;

        let is_buy = match order_params.side {
            OrderSide::BUY => true,
            OrderSide::SELL => false,
            _ => {
                return Err(InfraError::ApiCliError(
                    "Unsupported order side for Hyperliquid".into(),
                ));
            },
        };

        let price = order_params.price.ok_or(InfraError::ApiCliError(
            "Missing price for Hyperliquid limit order".into(),
        ))?;

        let tif = order_params.extra.get("tif").map(|s| s.as_str()).unwrap_or(
            match order_params.order_type {
                OrderType::PostOnly => "Alo",
                OrderType::Ioc => "Ioc",
                OrderType::Fok => "Ioc",
                _ => "Gtc",
            },
        );

        let reduce_only = order_params.reduce_only.unwrap_or(false);
        let grouping = order_params
            .extra
            .get("grouping")
            .map(|s| s.as_str())
            .unwrap_or("na");

        let cloid = order_params
            .client_order_id
            .or_else(|| order_params.extra.get("cloid").cloned());

        let order = if let Some(cloid) = cloid.as_ref() {
            json!({
                "a": asset,
                "b": is_buy,
                "p": price,
                "s": order_params.size,
                "r": reduce_only,
                "t": { "limit": { "tif": tif } },
                "c": cloid
            })
        } else {
            json!({
                "a": asset,
                "b": is_buy,
                "p": price,
                "s": order_params.size,
                "r": reduce_only,
                "t": { "limit": { "tif": tif } }
            })
        };

        let action = json!({
            "type": "order",
            "orders": [order],
            "grouping": grouping
        });

        let base_url = if self.is_testnet {
            HYPERLIQUID_TESTNET_BASE_URL
        } else {
            HYPERLIQUID_MAINNET_BASE_URL
        };

        let nonce = get_mills_timestamp();
        let vault_address = order_params.extra.get("vault_address").map(|s| s.as_str());
        let expires_after = order_params
            .extra
            .get("expires_after")
            .and_then(|s| s.parse::<u64>().ok());

        let res: RestResHyperliquid<RestOrderResponse> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(HyperliquidSignedRequest {
                client: &self.client,
                action: &action,
                nonce,
                vault_address,
                expires_after,
                is_mainnet: !self.is_testnet,
                base_url,
                endpoint: HYPERLIQUID_EXCHANGE_ENDPOINT,
            })
            .await?;

        let data = res.into_data()?;
        data.into_order_ack(cloid)
    }
}
