use std::sync::Arc;

use reqwest::Client;
use serde_json::{Value, json};
use simd_json::from_slice;

use crate::arch::{
    market_assets::{
        api_data::utils_data::InstrumentInfo,
        base_data::InstrumentType,
    },
    traits::market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
};
use crate::errors::{InfraError, InfraResult};

use super::{
    config_assets::{HYPERLIQUID_BASE_URL, HYPERLIQUID_INFO},
    schemas::rest::{
        meta::RestMetaHyperliquid,
        spot_meta::RestSpotMetaHyperliquid,
    },
};

#[derive(Clone, Debug)]
pub struct HyperliquidCli {
    pub client: Arc<Client>,
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
    fn init_api_key(&mut self) {}
}

impl LobWebsocket for HyperliquidCli {}

impl HyperliquidCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
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
                "Hyperliquid get_instrument_info currently supports Spot and Perpetual only"
                    .into(),
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
}
