use std::collections::HashSet;
use std::future::{ready, Future};
use std::sync::Arc;
use serde_json::{from_str, json};
use reqwest::Client;

use tracing::info;

use crate::errors::{InfraError, InfraResult};

use crate::market_assets::{
    api_general::RequestMethod,
    base_data::*,
    account_data::*,
    price_data::*
};
use crate::market_assets::cex::binance::config_assets::{BINANCE_UM_FUTURES_BASE_URL, BINANCE_UM_FUTURES_EXCHANGE_INFO};
use crate::market_assets::cex::prelude::BinanceUmCli;
use crate::task_execution::task_ws::*;

use crate::traits::{
    conversion::WsSubscribe,
    market_cex::{CexPrivateRest, CexPublicRest, MarketCexApi}
};

use super::{
    api_key::OkxKey,
    api_utils::*,
    config_assets::*,
    rest::{
        account_balance::AccountBalInfo,
    }
};

pub struct OkxCli {
    pub client: Client,
    pub api_key: Option<OkxKey>,
}

impl MarketCexApi for OkxCli {}

impl CexPublicRest for OkxCli {
}

impl CexPrivateRest for OkxCli {
    async fn get_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        self.get_balance(assets).await
    }
}


impl OkxCli {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: None
        }
    }

    async fn get_balance(
        &self,
        assets: Vec<String>,
    ) -> InfraResult<Vec<BalanceData>> {
        let api_key = self.api_key.as_ref().ok_or(InfraError::ApiNotInitialized)?;

        let body = json!({
            "ccy": assets,
        }).to_string();

        let all_balances: OkxRestRes<AccountBalInfo> = api_key
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_ACCOUNT_BALANCE,
            )
            .await?;

        let result: Vec<BalanceData> = all_balances
            .data
            .into_iter()
            .flat_map(|account| account.details)
            .map(BalanceData::from)
            .collect();

        Ok(result)
    }
}

