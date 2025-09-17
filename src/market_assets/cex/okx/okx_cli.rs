use serde_json::{from_str, json};
use reqwest::Client;


use crate::errors::{InfraError, InfraResult};

use crate::market_assets::{
    api_general::RequestMethod,
    account_data::*,
};
use crate::market_assets::{
    utils_data::PubCopytraderSubpositions,
};
use crate::traits::{
    market_cex::{CexPrivateRest, CexPublicRest, MarketCexApi}
};

use super::{
    api_key::OkxKey,
    api_utils::*,
    config_assets::*,
    rest::{
        account_balance::RestAccountBalOkx,
        public_current_subpositions::RestSubPositionOkx,
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

impl Default for OkxCli {
    fn default() -> Self {
        Self::new()
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
        let body = json!({
            "ccy": assets,
        }).to_string();

        let bal_res: RestResOkx<RestAccountBalOkx> = self.api_key
            .as_ref()
            .ok_or(InfraError::ApiNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_ACCOUNT_BALANCE,
            )
            .await?;

        if bal_res.code != "0" {
            let msg = bal_res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", bal_res.code, msg)));
        }

        let result: Vec<BalanceData> = bal_res
            .data
            .into_iter()
            .flat_map(|account| account.details)
            .map(BalanceData::from)
            .collect();

        Ok(result)
    }

    pub async fn get_pub_current_subpositions(
        &self,
        unique_code: &str,
        inst_type: Option<&str>,
        limit: Option<u32>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> InfraResult<Vec<PubCopytraderSubpositions>> {
        let inst_type = inst_type.unwrap_or("SWAP");

        let mut url = format!(
            "{}{}?uniqueCode={}&instType={}",
            OKX_BASE_URL,
            OKX_COPYTRADER_PUBLIC_SUBPOSITIONS,
            unique_code,
            inst_type
        );

        if let Some(l) = limit {
            url.push_str(&format!("&limit={}", l));
        }
        if let Some(b) = before {
            url.push_str(&format!("&before={}", b));
        }
        if let Some(a) = after {
            url.push_str(&format!("&after={}", a));
        }

        let text = self.client
            .get(&url)
            .send().await?
            .text().await?;

        let sub_pos_res: RestResOkx<RestSubPositionOkx> = from_str(&text)?;

        if sub_pos_res.code != "0" {
            let msg = sub_pos_res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", sub_pos_res.code, msg)));
        }

        let result: Vec<PubCopytraderSubpositions> = sub_pos_res
            .data
            .into_iter()
            .map(PubCopytraderSubpositions::from)
            .collect();

        Ok(result)
    }
}

