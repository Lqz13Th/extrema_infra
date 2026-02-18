use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::account_data::{BalanceData, BorrowableData},
        api_general::RequestMethod,
        exchange::gate::{
            config_assets::{
                GATE_BASE_URL, GATE_UNI_ACCOUNTS, GATE_UNI_BORROWABLE, GATE_UNI_CURRENCIES,
                GATE_UNI_ESTIMATE_RATE, GATE_UNI_LOANS,
            },
            gate_rest_msg::RestResGate,
            schemas::uni_rest::{
                account_balance::RestAccountBalGateUnified,
                borrowable::RestBorrowableGateUnified,
                currencies::RestCurrenciesGateUnified,
                loans::{RestLoanGateUnified, RestLoanTranGateUnified},
            },
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

    pub async fn get_borrowable(&self, inst: &str) -> InfraResult<Vec<BorrowableData>> {
        let query = format!("currency={}", inst);
        let res: RestResGate<RestBorrowableGateUnified> = self
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

    pub async fn get_currencies(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<RestCurrenciesGateUnified>> {
        let query = match inst {
            Some(inst) => format!("currency={}", inst),
            None => String::new(),
        };

        let res: RestResGate<RestCurrenciesGateUnified> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query),
                None,
                GATE_BASE_URL,
                GATE_UNI_CURRENCIES,
            )
            .await?;

        res.into_vec()
    }

    pub async fn post_loans(
        &self,
        currency: &str,
        loan_type: &str,
        amount: &str,
        repaid_all: Option<bool>,
        text: Option<&str>,
    ) -> InfraResult<Vec<RestLoanTranGateUnified>> {
        if loan_type != "borrow" && loan_type != "repay" {
            return Err(InfraError::ApiCliError(
                "Gate unified loans type must be borrow or repay".into(),
            ));
        }

        let mut body = json!({
            "currency": currency,
            "type": loan_type,
            "amount": amount,
        });

        if let Some(v) = repaid_all {
            body["repaid_all"] = json!(v);
        }

        if let Some(v) = text {
            body["text"] = json!(normalize_gate_text(v));
        }

        let res: RestResGate<RestLoanTranGateUnified> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Post,
                None,
                Some(&body.to_string()),
                GATE_BASE_URL,
                GATE_UNI_LOANS,
            )
            .await?;

        res.into_vec()
    }

    pub async fn get_loans(
        &self,
        currency: Option<&str>,
        page: Option<i32>,
        limit: Option<i32>,
        loan_type: Option<&str>,
    ) -> InfraResult<Vec<RestLoanGateUnified>> {
        if let Some(v) = loan_type
            && v != "platform"
            && v != "margin"
        {
            return Err(InfraError::ApiCliError(
                "Gate unified loans query type must be platform or margin".into(),
            ));
        }

        let mut query_parts: Vec<String> = Vec::new();

        if let Some(v) = currency {
            query_parts.push(format!("currency={}", v));
        }
        if let Some(v) = page {
            query_parts.push(format!("page={}", v));
        }
        if let Some(v) = limit {
            query_parts.push(format!("limit={}", v));
        }
        if let Some(v) = loan_type {
            query_parts.push(format!("type={}", v));
        }

        let query = query_parts.join("&");
        let query_opt = if query.is_empty() {
            None
        } else {
            Some(query.as_str())
        };

        let res: RestResGate<RestLoanGateUnified> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                query_opt,
                None,
                GATE_BASE_URL,
                GATE_UNI_LOANS,
            )
            .await?;

        res.into_vec()
    }

    pub async fn get_estimate_rate(&self, insts: &[String]) -> InfraResult<HashMap<String, f64>> {
        if insts.is_empty() {
            return Err(InfraError::ApiCliError(
                "Gate uni estimate_rate requires at least one currency".into(),
            ));
        }

        let currencies = insts.join(",");
        let query = format!("currencies={}", currencies);

        let res: RestResGate<HashMap<String, String>> = self
            .api_key
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                Some(&query),
                None,
                GATE_BASE_URL,
                GATE_UNI_ESTIMATE_RATE,
            )
            .await?;

        let raw_map = res.into_vec()?.into_iter().next().unwrap_or_default();

        let mut result: HashMap<String, f64> = HashMap::new();
        for (currency, rate_str) in raw_map {
            if rate_str.is_empty() {
                continue;
            }

            match rate_str.parse::<f64>() {
                Ok(rate) => {
                    result.insert(currency, rate);
                },
                Err(_) => {
                    error!(
                        "Failed to parse estimate rate for {}: {}",
                        currency, rate_str
                    );
                },
            }
        }

        Ok(result)
    }

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let res: RestResGate<RestAccountBalGateUnified> = self
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
