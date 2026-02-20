use reqwest::Client;
use simd_json::from_slice;
use std::sync::Arc;

use crate::arch::{
    market_assets::{
        api_data::utils_data::InstrumentInfo,
        base_data::{InstrumentType, TRADING_LOWER},
    },
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_key::{GateKey, read_gate_env_key},
    api_utils::gate_inst_to_cli,
    config_assets::{GATE_BASE_URL, GATE_DELIVERY_CONTRACTS},
    gate_rest_msg::RestResGate,
    schemas::delivery_rest::contract_delivery::RestContractGateDelivery,
};

use tracing::error;

#[derive(Clone, Debug)]
pub struct GateDeliveryCli {
    pub client: Arc<Client>,
    pub api_key: Option<GateKey>,
}

impl Default for GateDeliveryCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for GateDeliveryCli {}

impl LobPublicRest for GateDeliveryCli {
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

impl LobPrivateRest for GateDeliveryCli {
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
}

impl LobWebsocket for GateDeliveryCli {}

impl GateDeliveryCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    async fn _get_delivery_contracts(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<RestContractGateDelivery>> {
        let endpoint = GATE_DELIVERY_CONTRACTS.replace("{settle}", settle);

        let mut params: Vec<String> = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }

        let url = if params.is_empty() {
            [GATE_BASE_URL, &endpoint].concat()
        } else {
            format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"))
        };

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestContractGateDelivery> = from_slice(&mut res_bytes)?;

        res.into_vec()
    }

    async fn _get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        let mut data: Vec<InstrumentInfo> = Vec::new();

        if inst_type != InstrumentType::Futures {
            return Err(InfraError::ApiCliError(
                "Gate delivery cli only supports futures instruments".into(),
            ));
        }

        for settle in ["usdt", "btc"] {
            let contracts = self._get_delivery_contracts(settle, None, None).await?;
            data.extend(contracts.into_iter().map(InstrumentInfo::from));
        }

        Ok(data)
    }

    async fn _get_live_instruments(&self, inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        let mut data: Vec<String> = Vec::new();

        if inst_type != InstrumentType::Futures {
            return Err(InfraError::ApiCliError(
                "Gate delivery cli only supports futures instruments".into(),
            ));
        }

        for settle in ["usdt", "btc"] {
            let contracts = self._get_delivery_contracts(settle, None, None).await?;
            data.extend(
                contracts
                    .into_iter()
                    .filter(|c| {
                        if !c.status.is_empty() {
                            c.status.as_str() == TRADING_LOWER
                        } else {
                            !c.in_delisting
                        }
                    })
                    .map(|c| gate_inst_to_cli(&c.name)),
            );
        }

        Ok(data)
    }
}
