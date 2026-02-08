use reqwest::Client;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::account_data::{BalanceData, BorrowableData},
        api_general::RequestMethod,
        exchange::gate::{
            config_assets::{GATE_BASE_URL, GATE_UNI_ACCOUNTS, GATE_UNI_BORROWABLE},
            gate_rest_msg::RestResGate,
            schemas::uni_rest::{
                account_balance::RestAccountBalGate, borrowable::RestBorrowableGate,
            },
        },
    },
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::api_key::{GateKey, read_gate_env_key};

#[derive(Clone, Debug)]
pub struct GateUniCli {
    pub client: Arc<Client>,
    pub api_key: Option<GateKey>,
}

impl Default for GateUniCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for GateUniCli {}

impl LobPublicRest for GateUniCli {}

impl LobPrivateRest for GateUniCli {
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

impl LobWebsocket for GateUniCli {}

impl GateUniCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    pub async fn get_borrowable(&self, currency: &str) -> InfraResult<Vec<BorrowableData>> {
        let query = format!("currency={}", currency);
        let res: RestResGate<RestBorrowableGate> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query),
                None,
                GATE_BASE_URL,
                GATE_UNI_BORROWABLE,
            )
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(BorrowableData::from)
            .collect();

        Ok(data)
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
                GATE_UNI_ACCOUNTS,
            )
            .await?;

        let balances: Vec<BalanceData> = res
            .into_vec()?
            .into_iter()
            .flat_map(Vec::<BalanceData>::from)
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
