use reqwest::Client;
use serde_json::{Value, json};
use simd_json::from_slice;
use std::{collections::HashMap, sync::Arc};
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{account_data::OrderAckData, utils_data::InstrumentInfo},
        api_general::{OrderParams, get_mills_timestamp},
        base_data::InstrumentType,
    },
    task_execution::task_ws::{TradesParam, WsChannel},
    traits::market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_utils::*,
    auth::{HyperliquidAuth, read_hyperliquid_env_auth},
    config_assets::*,
    schemas::rest::{
        meta::RestMetaHyperliquid, spot_meta::RestSpotMetaHyperliquid,
        trade_order::RestOrderAckHyperliquid,
    },
};

#[derive(Clone, Debug)]
pub struct HyperliquidCli {
    pub client: Arc<Client>,
    pub auth: Option<HyperliquidAuth>,
    pub inst_index_map: HashMap<String, u32>,
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

impl LobWebsocket for HyperliquidCli {
    async fn get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        self._get_public_sub_msg(channel, insts)
    }

    async fn get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match channel {
            WsChannel::Trades(Some(TradesParam::AggTrades))
            | WsChannel::Trades(Some(TradesParam::AllTrades))
            | WsChannel::Trades(None) => Ok(HYPERLIQUID_WS.into()),
            _ => Err(InfraError::Unimplemented),
        }
    }
}

impl HyperliquidCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            auth: None,
            inst_index_map: HashMap::new(),
        }
    }

    pub async fn init_inst_index_map(&mut self) -> InfraResult<()> {
        let mut inst_index_map = HashMap::new();

        for inst_info in self._get_instrument_info(InstrumentType::Perpetual).await? {
            let inst = inst_info.inst;
            let index = hyperliquid_asset_id_to_index(
                InstrumentType::Perpetual,
                inst_info.inst_code.as_deref().ok_or_else(|| {
                    InfraError::ApiCliError(format!(
                        "Hyperliquid perpetual instrument missing inst_code: {}",
                        inst
                    ))
                })?,
            )?;

            if inst_index_map.insert(inst.clone(), index).is_some() {
                return Err(InfraError::ApiCliError(format!(
                    "Duplicate Hyperliquid instrument in inst_index_map: {}",
                    inst
                )));
            }
        }

        for inst_info in self._get_instrument_info(InstrumentType::Spot).await? {
            let inst = inst_info.inst;
            let index = hyperliquid_asset_id_to_index(
                InstrumentType::Spot,
                inst_info.inst_code.as_deref().ok_or_else(|| {
                    InfraError::ApiCliError(format!(
                        "Hyperliquid spot instrument missing inst_code: {}",
                        inst
                    ))
                })?,
            )?;

            if inst_index_map.insert(inst.clone(), index).is_some() {
                return Err(InfraError::ApiCliError(format!(
                    "Duplicate Hyperliquid instrument in inst_index_map: {}",
                    inst
                )));
            }
        }

        self.inst_index_map = inst_index_map;

        Ok(())
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
        let mut order_params = order_params;
        let asset_id = self.inst_to_asset_id(&order_params.inst)?;
        order_params.inst = asset_id.to_string();

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

    fn inst_to_asset_id(&self, inst: &str) -> InfraResult<u32> {
        let index = self.inst_index_map.get(inst).copied().ok_or_else(|| {
            if self.inst_index_map.is_empty() {
                InfraError::ApiCliError(
                    "Hyperliquid inst_index_map is empty, call init_inst_index_map() first".into(),
                )
            } else {
                InfraError::ApiCliError(format!(
                    "Hyperliquid inst not found in inst_index_map: {}",
                    inst
                ))
            }
        })?;

        let inst_type = if inst.ends_with(&format!("_{}_PERP", HYPERLIQUID_QUOTE)) {
            InstrumentType::Perpetual
        } else {
            InstrumentType::Spot
        };

        hyperliquid_index_to_asset_id(inst_type, index)
    }

    fn _get_public_sub_msg(
        &self,
        channel: &WsChannel,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        match channel {
            WsChannel::Trades(Some(TradesParam::AggTrades))
            | WsChannel::Trades(Some(TradesParam::AllTrades))
            | WsChannel::Trades(None) => self._ws_subscribe_trades(insts),
            _ => Err(InfraError::Unimplemented),
        }
    }

    fn _ws_subscribe_trades(&self, insts: Option<&[String]>) -> InfraResult<String> {
        let insts = insts.ok_or_else(|| {
            InfraError::ApiCliError("Hyperliquid trades ws requires at least one instrument".into())
        })?;

        if insts.is_empty() {
            return Err(InfraError::ApiCliError(
                "Hyperliquid trades ws requires at least one instrument".into(),
            ));
        }

        let msgs: InfraResult<Vec<String>> = insts
            .iter()
            .map(|inst| {
                let coin = self._inst_to_trade_coin(inst)?;
                Ok(ws_subscribe_msg_hyperliquid_trades(&coin))
            })
            .collect();

        Ok(msgs?.join("\n"))
    }

    fn _inst_to_trade_coin(&self, inst: &str) -> InfraResult<String> {
        let index = self.inst_index_map.get(inst).copied().ok_or_else(|| {
            if self.inst_index_map.is_empty() {
                InfraError::ApiCliError(
                    "Hyperliquid inst_index_map is empty, call init_inst_index_map() first".into(),
                )
            } else {
                InfraError::ApiCliError(format!(
                    "Hyperliquid inst not found in inst_index_map: {}",
                    inst
                ))
            }
        })?;

        if let Some(coin) = inst.strip_suffix(&format!("_{}_PERP", HYPERLIQUID_QUOTE)) {
            return Ok(coin.to_string());
        }

        if inst == "PURR_USDC" {
            return Ok("PURR/USDC".into());
        }

        Ok(format!("@{}", index))
    }
}
