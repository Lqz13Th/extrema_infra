use reqwest::Client;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{api_data::account_data::BalanceData, api_general::RequestMethod},
    traits::{
        conversion::IntoInfraVec,
        market_cex::{CexPrivateRest, CexPublicRest, CexWebsocket, MarketCexApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{GateKey, read_gate_env_key},
    config_assets::*,
    gate_rest_msg::RestResGate,
    schemas::rest::account_balance::RestAccountBalGate,
};

#[derive(Clone, Debug)]
pub struct GateCli {
    pub client: Arc<Client>,
    pub api_key: Option<GateKey>,
}

impl Default for GateCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketCexApi for GateCli {}

impl CexPublicRest for GateCli {}

impl CexPrivateRest for GateCli {
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

    async fn get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
    }
}

impl CexWebsocket for GateCli {}

impl GateCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let res: RestResGate<RestAccountBalGate> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                None,
                None,
                GATE_BASE_URL,
                GATE_UNIFIED_ACCOUNTS,
            )
            .await?;

        let balances: Vec<BalanceData> = res
            .into_vec()?
            .into_iter()
            .flat_map(|account| account.into_balance_vec())
            .collect();

        let filtered = match assets {
            Some(list) if !list.is_empty() => balances
                .into_iter()
                .filter(|b| list.contains(&b.asset))
                .collect(),
            _ => balances,
        };

        Ok(filtered)
    }
}
