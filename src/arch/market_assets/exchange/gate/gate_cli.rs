use reqwest::Client;
use simd_json::from_slice;
use std::sync::Arc;
use tracing::error;

use super::{
    api_key::{GateKey, read_gate_env_key},
    api_utils::{cli_perp_to_gate_inst, gate_inst_to_cli},
    config_assets::*,
    gate_rest_msg::RestResGate,
    schemas::rest::{
        account_balance::RestAccountBalGate, contract_delivery::RestDeliveryContractGate,
        contract_futures::RestContractGate, funding_rate::RestFundingRateGate,
    },
};
use crate::arch::{
    market_assets::{
        api_data::{
            account_data::BalanceData,
            utils_data::{FundingRateData, InstrumentInfo},
        },
        api_general::RequestMethod,
        base_data::InstrumentType,
    },
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};
use crate::prelude::TRADING_LOWER;

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

impl MarketLobApi for GateCli {}

impl LobPublicRest for GateCli {
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

impl LobPrivateRest for GateCli {
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

impl LobWebsocket for GateCli {}

impl GateCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            api_key: None,
        }
    }

    pub async fn get_funding_rate_history(
        &self,
        settle: &str,
        inst: &str,
        limit: Option<u32>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let endpoint = GATE_FUTURES_FUNDING_RATE.replace("{settle}", settle);

        let mut params: Vec<String> = Vec::new();
        params.push(format!("contract={}", cli_perp_to_gate_inst(inst)));
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(s) = start_time {
            params.push(format!("from={}", s));
        }
        if let Some(e) = end_time {
            params.push(format!("to={}", e));
        }

        let url = if params.is_empty() {
            [GATE_BASE_URL, &endpoint].concat()
        } else {
            format!("{}{}?{}", GATE_BASE_URL, endpoint, params.join("&"))
        };

        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestFundingRateGate> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_funding_rate_data(inst))
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_live_all(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let endpoint = GATE_FUTURES_CONTRACTS.replace("{settle}", settle);

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
        let res: RestResGate<RestContractGate> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_funding_rate_data())
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_live(
        &self,
        settle: &str,
        inst: &str,
    ) -> InfraResult<Vec<FundingRateData>> {
        let endpoint = GATE_FUTURES_CONTRACT
            .replace("{settle}", settle)
            .replace("{contract}", &cli_perp_to_gate_inst(inst));

        let url = [GATE_BASE_URL, &endpoint].concat();
        let responds = self.client.get(url).send().await?;
        let mut res_bytes = responds.bytes().await?.to_vec();
        let res: RestResGate<RestContractGate> = from_slice(&mut res_bytes)?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_funding_rate_data())
            .collect();

        Ok(data)
    }

    async fn _get_futures_contracts(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<RestContractGate>> {
        let endpoint = GATE_FUTURES_CONTRACTS.replace("{settle}", settle);

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
        let res: RestResGate<RestContractGate> = from_slice(&mut res_bytes)?;

        res.into_vec()
    }

    async fn _get_delivery_contracts(
        &self,
        settle: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> InfraResult<Vec<RestDeliveryContractGate>> {
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
        let res: RestResGate<RestDeliveryContractGate> = from_slice(&mut res_bytes)?;

        res.into_vec()
    }

    async fn _get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        let mut data: Vec<InstrumentInfo> = Vec::new();

        for settle in ["usdt", "btc"] {
            match inst_type {
                InstrumentType::Perpetual => {
                    let contracts = self._get_futures_contracts(settle, None, None).await?;
                    data.extend(contracts.into_iter().map(InstrumentInfo::from));
                },
                InstrumentType::Futures => {
                    let contracts = self._get_delivery_contracts(settle, None, None).await?;
                    data.extend(contracts.into_iter().map(InstrumentInfo::from));
                },
                _ => {
                    return Err(InfraError::ApiCliError(
                        "Gate only supports futures/perpetual instruments".into(),
                    ));
                },
            }
        }

        Ok(data)
    }

    async fn _get_live_instruments(&self, inst_type: InstrumentType) -> InfraResult<Vec<String>> {
        let mut data: Vec<String> = Vec::new();

        for settle in ["usdt", "btc"] {
            match inst_type {
                InstrumentType::Perpetual => {
                    let contracts = self._get_futures_contracts(settle, None, None).await?;
                    data.extend(
                        contracts
                            .into_iter()
                            .filter(|c| c.status.as_str() == TRADING_LOWER)
                            .map(|c| gate_inst_to_cli(&c.name)),
                    );
                },
                InstrumentType::Futures => {
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
                },
                _ => {
                    return Err(InfraError::ApiCliError(
                        "Gate only supports futures/perpetual instruments".into(),
                    ));
                },
            }
        }

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
