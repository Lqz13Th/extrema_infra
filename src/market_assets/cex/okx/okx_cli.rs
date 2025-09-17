use serde_json::{from_str, json};
use reqwest::Client;
use tracing::error;

use crate::errors::{InfraError, InfraResult};
use crate::market_assets::{
    api_general::RequestMethod,
    base_data::InstrumentType,
    account_data::*,
    utils_data::*,
};
use crate::market_assets::cex::okx::rest::public_lead_traders::RestPubLeadTradersOkx;
use crate::traits::{
    market_cex::{CexPrivateRest, CexPublicRest, MarketCexApi}
};

use super::{
    api_key::{OkxKey, read_okx_env_key},
    api_utils::*,
    config_assets::*,
    rest::{
        account_balance::RestAccountBalOkx,
        current_lead_traders::RestLeadtraderOkx,
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
    fn init_api_key(&mut self) {
        match read_okx_env_key() {
            Ok(okx_key) => {
                self.api_key = Some(okx_key);
            },
            Err(e) => {
                error!("Failed to read OKX env key: {:?}", e);
            }
        }
    }

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

        let all_balances: Vec<BalanceData> = bal_res
            .data
            .into_iter()
            .flat_map(|account| account.details)
            .map(BalanceData::from)
            .collect();

        let filtered = if assets.is_empty() {
            all_balances
        } else {
            all_balances
                .into_iter()
                .filter(|b| assets.contains(&b.asset))
                .collect()
        };

        Ok(filtered)
    }

    pub async fn get_lead_traders(
        &self,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<CurrentLeadtrader>> {
        let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Spot => "SPOT",
            InstrumentType::Perpetual => "SWAP",
            InstrumentType::Option => "OPTION",
        };

        let body = json!({
            "instType": inst_type_str,
        }).to_string();

        let res: RestResOkx<RestLeadtraderOkx> = self.api_key
            .as_ref()
            .ok_or(InfraError::ApiNotInitialized)?
            .send_signed_request(
                &self.client,
                RequestMethod::Get,
                body,
                OKX_BASE_URL,
                OKX_CURRENT_LEADTRADERS,
            )
            .await?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let result: Vec<CurrentLeadtrader> = res
            .data
            .into_iter()
            .map(CurrentLeadtrader::from)
            .collect();

        Ok(result)
    }

    pub async fn get_public_lead_traders(
        &self,

    )  {

        let mut url = format!(
            "{}{}",
            OKX_BASE_URL,
            OKX_PUBLIC_LEADTRADERS,
        );


        let text = self.client
            .get(&url)
            .send().await.unwrap()
            .text().await.unwrap();

        println!("{}", text);


    }

    pub async fn get_lead_trader_subpositions(
        &self,
        unique_code: &str,
        inst_type: Option<InstrumentType>,
        limit: Option<u32>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> InfraResult<Vec<LeadtraderSubpositions>> {
        // let inst_type_str = match inst_type.unwrap_or(InstrumentType::Perpetual) {
        //     InstrumentType::Spot => "SPOT",
        //     InstrumentType::Perpetual => "SWAP",
        //     InstrumentType::Option => "OPTION",
        // };

        let mut url = format!(
            "{}{}?uniqueCode={}&instType={}",
            OKX_BASE_URL,
            OKX_LEADTRADER_SUBPOSITIONS,
            unique_code,
            "SWAP",
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

        println!("text: {}", text);
        let res: RestResOkx<RestSubPositionOkx> = from_str(&text)?;

        if res.code != "0" {
            let msg = res.msg.unwrap_or_default();
            return Err(InfraError::ApiError(format!("code {}: {}", res.code, msg)));
        }

        let result: Vec<LeadtraderSubpositions> = res
            .data
            .into_iter()
            .map(LeadtraderSubpositions::from)
            .collect();

        Ok(result)
    }
}

