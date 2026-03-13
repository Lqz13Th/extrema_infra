use reqwest::Client;
use serde_json::{Value, json};
use simd_json::from_slice;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{account_data::OrderAckData, utils_data::InstrumentInfo},
        api_general::{OrderParams, get_mills_timestamp},
        base_data::InstrumentType,
    },
    traits::market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_utils::*,
    auth::{HyperliquidAuth, read_hyperliquid_env_auth},
    config_assets::{HYPERLIQUID_BASE_URL, HYPERLIQUID_GROUPING_NA, HYPERLIQUID_INFO},
    schemas::rest::{
        meta::RestMetaHyperliquid, spot_meta::RestSpotMetaHyperliquid,
        trade_order::RestOrderAckHyperliquid,
    },
};

#[derive(Clone, Debug)]
pub struct HyperliquidCli {
    pub client: Arc<Client>,
    pub auth: Option<HyperliquidAuth>,
}

impl Default for HyperliquidCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for HyperliquidCli {}

impl LobPublicRest for HyperliquidCli {
    async fn get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        self._get_instrument_info(inst_type).await
    }
}

impl LobPrivateRest for HyperliquidCli {
    fn init_api_key(&mut self) {
        match read_hyperliquid_env_auth() {
            Ok(auth) => {
                self.auth = Some(auth);
            },
            Err(e) => {
                error!("Failed to read HYPERLIQUID env auth: {:?}", e);
            },
        }
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
            auth: None,
        }
    }

    async fn _get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        match inst_type {
            InstrumentType::Perpetual => {
                let body = json!({ "type": "meta" });
                let res: RestMetaHyperliquid = self.post_info(&body).await?;

                Ok(res.into_instrument_info())
            },
            InstrumentType::Spot => {
                let body = json!({ "type": "spotMeta" });
                let res: RestSpotMetaHyperliquid = self.post_info(&body).await?;

                Ok(res.into_instrument_info())
            },
            _ => Err(InfraError::ApiCliError(
                "Hyperliquid get_instrument_info currently supports Spot and Perpetual only".into(),
            )),
        }
    }

    async fn post_info<T>(&self, body: &Value) -> InfraResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", HYPERLIQUID_BASE_URL, HYPERLIQUID_INFO);
        let responds = self.client.post(url).json(body).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: T = from_slice(&mut res_bytes)?;

        Ok(res)
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let nonce = get_mills_timestamp();
        let action = HyperliquidOrderAction {
            kind: "order",
            orders: vec![hyperliquid_order_from_params(order_params)?],
            grouping: HYPERLIQUID_GROUPING_NA,
        };

        let data: Vec<RestOrderAckHyperliquid> = self
            .auth
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_exchange_action(&self.client, &action, nonce)
            .await?;

        let data: OrderAckData =
            data.into_iter()
                .map(OrderAckData::from)
                .next()
                .ok_or(InfraError::ApiCliError(
                    "No Hyperliquid order ack data returned".into(),
                ))?;

        Ok(data)
    }
}
