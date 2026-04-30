use reqwest::Client;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{
            account_data::{BalanceData, HistoOrderData, OrderAckData, PositionData},
            price_data::TickerData,
            utils_data::{FundingRateData, FundingRateInfo, InstrumentInfo},
        },
        api_general::{OrderParams, get_micros_timestamp, parse_json_response},
        base_data::{InstrumentType, MarginMode},
    },
    task_execution::task_ws::{TradesParam, WsChannel},
    traits::{
        conversion::IntoInfraVec,
        market_lob::{LobPrivateRest, LobPublicRest, LobWebsocket, MarketLobApi},
    },
};
use crate::errors::{InfraError, InfraResult};

use super::{
    api_utils::*,
    auth::{HyperliquidAuth, read_hyperliquid_env_auth},
    config_assets::*,
    hyperliquid_rest_msg::RestResHyperliquid,
    schemas::rest::{
        all_mids::RestAllMidsHyperliquid, asset_ctxs::RestMetaAndAssetCtxsHyperliquid,
        clearinghouse_state::RestClearinghouseStateHyperliquid,
        funding_history::RestFundingHistoryHyperliquid, meta::RestMetaHyperliquid,
        order_status::RestOrderStatusHyperliquid,
        spot_clearinghouse_state::RestSpotClearinghouseStateHyperliquid,
        spot_meta::RestSpotMetaHyperliquid, trade_order::RestOrderAckHyperliquid,
    },
};

#[derive(Clone, Debug)]
pub struct HyperliquidCli {
    pub client: Arc<Client>,
    pub auth: Option<HyperliquidAuth>,
    pub inst_index_map: HashMap<String, u32>,
    pub default_perp_dex: Option<String>,
}

impl Default for HyperliquidCli {
    fn default() -> Self {
        Self::new(Arc::new(Client::new()))
    }
}

impl MarketLobApi for HyperliquidCli {}

impl LobPublicRest for HyperliquidCli {
    async fn get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        self._get_tickers(insts, inst_type).await
    }

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

    async fn get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        self._get_balance(assets).await
    }

    async fn get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        self._get_positions(insts).await
    }

    async fn get_order_history(
        &self,
        inst: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<&str>,
    ) -> InfraResult<Vec<HistoOrderData>> {
        self._get_order_history(inst, start_time, end_time, limit, order_id)
            .await
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

    async fn get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_private_sub_msg(channel)
    }

    async fn get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_public_connect_msg(channel)
    }

    async fn get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        self._get_private_connect_msg(channel)
    }
}

impl HyperliquidCli {
    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            auth: None,
            inst_index_map: HashMap::new(),
            default_perp_dex: None,
        }
    }

    pub fn set_perp_dex(&mut self, dex: Option<String>) {
        self.default_perp_dex = dex.and_then(|dex| {
            let dex = dex.trim().to_string();
            (!dex.is_empty()).then_some(dex)
        });
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

    pub async fn get_funding_rate_live(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let target_inst = normalize_funding_inst_filter(inst)?;

        let data = self
            ._get_meta_and_asset_ctxs()
            .await?
            .into_funding_rate_data()?
            .into_iter()
            .filter(|entry| match &target_inst {
                Some(inst) => entry.inst == *inst,
                None => true,
            })
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_info(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<FundingRateInfo>> {
        let target_inst = normalize_funding_inst_filter(inst)?;

        let data = self
            ._get_meta_and_asset_ctxs()
            .await?
            .into_funding_rate_info()?
            .into_iter()
            .filter(|entry| match &target_inst {
                Some(inst) => entry.inst == *inst,
                None => true,
            })
            .collect();

        Ok(data)
    }

    pub async fn get_funding_rate_history(
        &self,
        inst: &str,
        start_time_ms: u64,
        end_time_ms: Option<u64>,
    ) -> InfraResult<Vec<FundingRateData>> {
        if let Some(end_time_ms) = end_time_ms
            && end_time_ms < start_time_ms
        {
            return Err(InfraError::ApiCliError(format!(
                "Hyperliquid funding history end_time_ms {} is earlier than start_time_ms {}",
                end_time_ms, start_time_ms
            )));
        }

        let coin = hyperliquid_cli_inst_to_raw_perp_coin(inst)?;
        let mut body = json!({
            "type": "fundingHistory",
            "coin": coin,
            "startTime": start_time_ms,
        });

        if let Some(end_time_ms) = end_time_ms {
            body["endTime"] = json!(end_time_ms);
        }

        let res: RestResHyperliquid<RestFundingHistoryHyperliquid> =
            self._post_info_raw(&body).await?;

        let data = res
            .into_vec()?
            .into_iter()
            .map(FundingRateData::from)
            .collect();

        Ok(data)
    }

    pub async fn get_perps_at_open_interest_cap(&self) -> InfraResult<Vec<String>> {
        let body = json!({
            "type": "perpsAtOpenInterestCap",
            "dex": self._perp_dex(),
        });
        let res: RestResHyperliquid<String> = self._post_info_raw(&body).await?;
        res.into_vec()
    }

    pub async fn set_leverage(
        &self,
        inst: &str,
        leverage: u32,
        margin_mode: MarginMode,
    ) -> InfraResult<()> {
        if leverage == 0 {
            return Err(InfraError::ApiCliError(
                "Hyperliquid leverage must be greater than 0".into(),
            ));
        }

        if !inst.ends_with(HYPERLIQUID_PERP_SUFFIX) {
            return Err(InfraError::ApiCliError(
                "Hyperliquid set_leverage supports perpetual instruments only".into(),
            ));
        }

        let action = HyperliquidUpdateLeverageAction {
            kind: "updateLeverage",
            asset: self._inst_to_asset_id(inst)?,
            is_cross: match margin_mode {
                MarginMode::Cross => true,
                MarginMode::Isolated => false,
                MarginMode::Unknown => {
                    return Err(InfraError::ApiCliError(
                        "Unknown margin mode for Hyperliquid set_leverage".into(),
                    ));
                },
            },
            leverage,
        };

        self.auth
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_exchange_action_raw::<Value, _>(&self.client, &action)
            .await?;

        Ok(())
    }

    async fn _get_instrument_info(
        &self,
        inst_type: InstrumentType,
    ) -> InfraResult<Vec<InstrumentInfo>> {
        match inst_type {
            InstrumentType::Perpetual => {
                let body = json!({ "type": "meta", "dex": self._perp_dex() });
                let res: RestResHyperliquid<RestMetaHyperliquid> =
                    self._post_info_raw(&body).await?;

                let data = res
                    .into_vec()?
                    .into_iter()
                    .next()
                    .ok_or(InfraError::ApiCliError(
                        "No Hyperliquid perpetual instrument info returned".into(),
                    ))?;

                Ok(data.into_instrument_info())
            },
            InstrumentType::Spot => {
                let body = json!({ "type": "spotMeta" });
                let res: RestResHyperliquid<RestSpotMetaHyperliquid> =
                    self._post_info_raw(&body).await?;

                let data = res
                    .into_vec()?
                    .into_iter()
                    .next()
                    .ok_or(InfraError::ApiCliError(
                        "No Hyperliquid spot instrument info returned".into(),
                    ))?;

                Ok(data.into_instrument_info())
            },
            _ => Err(InfraError::ApiCliError(
                "Hyperliquid get_instrument_info currently supports Spot and Perpetual only".into(),
            )),
        }
    }

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Perpetual => {
                let body = json!({ "type": "allMids", "dex": self._perp_dex() });
                let res: RestResHyperliquid<RestAllMidsHyperliquid> =
                    self._post_info_raw(&body).await?;

                let data = res
                    .into_vec()?
                    .into_iter()
                    .next()
                    .ok_or(InfraError::ApiCliError(
                        "No Hyperliquid allMids data returned".into(),
                    ))?
                    .into_perp_ticker_data(get_micros_timestamp())
                    .into_iter()
                    .filter(|ticker| match insts {
                        Some(list) => list.contains(&ticker.inst),
                        None => true,
                    })
                    .collect();

                Ok(data)
            },
            InstrumentType::Spot => {
                let mids_body = json!({ "type": "allMids" });
                let mids_res: RestResHyperliquid<RestAllMidsHyperliquid> =
                    self._post_info_raw(&mids_body).await?;
                let mids =
                    mids_res
                        .into_vec()?
                        .into_iter()
                        .next()
                        .ok_or(InfraError::ApiCliError(
                            "No Hyperliquid allMids data returned".into(),
                        ))?;

                let meta_body = json!({ "type": "spotMeta" });
                let meta_res: RestResHyperliquid<RestSpotMetaHyperliquid> =
                    self._post_info_raw(&meta_body).await?;
                let meta =
                    meta_res
                        .into_vec()?
                        .into_iter()
                        .next()
                        .ok_or(InfraError::ApiCliError(
                            "No Hyperliquid spot instrument info returned".into(),
                        ))?;

                let spot_inst_by_coin = meta.into_spot_inst_by_coin();
                let data = mids
                    .into_spot_ticker_data(get_micros_timestamp(), &spot_inst_by_coin)
                    .into_iter()
                    .filter(|ticker| match insts {
                        Some(list) => list.contains(&ticker.inst),
                        None => true,
                    })
                    .collect();

                Ok(data)
            },
            _ => Err(InfraError::ApiCliError(
                "Hyperliquid get_tickers currently supports Spot and Perpetual only".into(),
            )),
        }
    }

    async fn _place_order(&self, order_params: OrderParams) -> InfraResult<OrderAckData> {
        let mut order_params = order_params;
        let asset_id = self._inst_to_asset_id(&order_params.inst)?;
        order_params.inst = asset_id.to_string();

        let action = HyperliquidOrderAction {
            kind: "order",
            orders: vec![hyperliquid_order_from_params(order_params)?],
            grouping: HYPERLIQUID_GROUPING_NA,
        };

        let res: RestResHyperliquid<RestOrderAckHyperliquid> = self
            .auth
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)?
            .send_signed_exchange_action_raw(&self.client, &action)
            .await?;

        let data: OrderAckData = res
            .into_vec()?
            .into_iter()
            .map(OrderAckData::from)
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid order ack data returned".into(),
            ))?;

        Ok(data)
    }

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let user = self._owner_address()?;
        let res: RestResHyperliquid<RestSpotClearinghouseStateHyperliquid> = self
            ._post_info_raw(&json!({
                "type": "spotClearinghouseState",
                "user": user,
            }))
            .await?;

        let res = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid spotClearinghouseState returned".into(),
            ))?;

        let balances = res
            .into_balance_data()
            .into_iter()
            .filter(|balance| match &normalize_asset_filters(assets) {
                Some(assets) => assets.contains(&balance.asset),
                None => true,
            })
            .collect();

        Ok(balances)
    }

    async fn _get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        let user = self._owner_address()?;
        let res: RestResHyperliquid<RestClearinghouseStateHyperliquid> = self
            ._post_info_raw(&json!({
                "type": "clearinghouseState",
                "user": user,
                "dex": self._perp_dex(),
            }))
            .await?;

        let data = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid clearinghouse state returned".into(),
            ))?;

        let normalized_insts = normalize_inst_filters(insts);
        let mark_px_by_coin = self
            ._get_meta_and_asset_ctxs()
            .await?
            .into_perp_mark_px_by_coin()?;

        let positions = data
            .into_position_data(&mark_px_by_coin)
            .into_iter()
            .filter(|position| match &normalized_insts {
                Some(insts) => insts.contains(&position.inst),
                None => true,
            })
            .collect();

        Ok(positions)
    }

    async fn _get_order_history(
        &self,
        inst: &str,
        _start_time: Option<u64>,
        _end_time: Option<u64>,
        limit: Option<u32>,
        order_id: Option<&str>,
    ) -> InfraResult<Vec<HistoOrderData>> {
        let user = self._owner_address()?;
        let body = match order_id {
            Some(order_id) => json!({
                "type": "orderStatus",
                "user": user,
                "oid": order_id.parse::<u64>().map_err(|_| {
                    InfraError::ApiCliError(format!(
                        "Invalid Hyperliquid order_id, expected u64 string: {}",
                        order_id
                    ))
                })?,
            }),
            None => json!({
                "type": "historicalOrders",
                "user": user,
            }),
        };

        let normalized_inst = normalize_hyperliquid_cli_inst(inst);

        let res: RestResHyperliquid<RestOrderStatusHyperliquid> =
            self._post_info_raw(&body).await?;

        let mut data: Vec<HistoOrderData> = res
            .into_vec()?
            .into_iter()
            .map(HistoOrderData::from)
            .filter(|order| order.inst == normalized_inst)
            .collect();

        data.sort_by_key(|b| std::cmp::Reverse(b.update_time));
        if let Some(limit) = limit {
            data.truncate(limit as usize);
        }

        Ok(data)
    }

    fn _get_public_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match channel {
            WsChannel::Trades(Some(TradesParam::AggTrades))
            | WsChannel::Trades(Some(TradesParam::AllTrades))
            | WsChannel::Trades(None) => Ok(HYPERLIQUID_WS.into()),
            _ => Err(InfraError::Unimplemented),
        }
    }

    fn _get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match channel {
            WsChannel::AccountOrders | WsChannel::AccountPositions => Ok(HYPERLIQUID_WS.into()),
            _ => Err(InfraError::Unimplemented),
        }
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

    fn _get_private_sub_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        let user = self._owner_address()?;

        match channel {
            WsChannel::AccountOrders => Ok(json!({
                "method": "subscribe",
                "subscription": {
                    "type": "orderUpdates",
                    "user": user,
                }
            })
            .to_string()),
            WsChannel::AccountPositions => Ok(json!({
                "method": "subscribe",
                "subscription": {
                    "type": "clearinghouseState",
                    "user": user,
                    "dex": self._perp_dex(),
                }
            })
            .to_string()),
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
                Ok(json!({
                    "method": "subscribe",
                    "subscription": {
                        "type": "trades",
                        "coin": coin.to_string(),
                    }
                })
                .to_string())
            })
            .collect();

        Ok(msgs?.join("\n"))
    }

    fn _inst_to_trade_coin(&self, inst: &str) -> InfraResult<String> {
        if let Some(coin) = hyperliquid_cli_inst_to_raw_trade_coin(inst) {
            return Ok(coin);
        }

        let normalized_inst = normalize_hyperliquid_cli_inst(inst);
        let index = self
            .inst_index_map
            .get(&normalized_inst)
            .copied()
            .ok_or_else(|| {
                if self.inst_index_map.is_empty() {
                    InfraError::ApiCliError(
                        "Hyperliquid inst_index_map is empty, call init_inst_index_map() first"
                            .into(),
                    )
                } else {
                    InfraError::ApiCliError(format!(
                        "Hyperliquid inst not found in inst_index_map: {}",
                        inst
                    ))
                }
            })?;

        Ok(format!("@{}", index))
    }

    fn _owner_address(&self) -> InfraResult<&str> {
        self.auth
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)
            .map(|auth| auth.owner_address.as_str())
    }

    fn _perp_dex(&self) -> &str {
        self.default_perp_dex.as_deref().unwrap_or("")
    }

    async fn _post_info_raw<T>(&self, body: &Value) -> InfraResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = [HYPERLIQUID_BASE_URL, HYPERLIQUID_INFO].concat();
        let response = self.client.post(url).json(body).send().await?;
        let info_type = body.get("type").and_then(|v| v.as_str()).unwrap_or("?");
        let label = format!("Hyperliquid info {}", info_type);
        parse_json_response(&label, response).await
    }

    async fn _get_meta_and_asset_ctxs(&self) -> InfraResult<RestMetaAndAssetCtxsHyperliquid> {
        let ctxs_res: RestResHyperliquid<RestMetaAndAssetCtxsHyperliquid> = self
            ._post_info_raw(&json!({ "type": "metaAndAssetCtxs", "dex": self._perp_dex() }))
            .await?;

        ctxs_res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid metaAndAssetCtxs returned".into(),
            ))
    }

    fn _inst_to_asset_id(&self, inst: &str) -> InfraResult<u32> {
        let normalized_inst = normalize_hyperliquid_cli_inst(inst);
        let index = self
            .inst_index_map
            .get(&normalized_inst)
            .copied()
            .ok_or_else(|| {
                if self.inst_index_map.is_empty() {
                    InfraError::ApiCliError(
                        "Hyperliquid inst_index_map is empty, call init_inst_index_map() first"
                            .into(),
                    )
                } else {
                    InfraError::ApiCliError(format!(
                        "Hyperliquid inst not found in inst_index_map: {}",
                        inst
                    ))
                }
            })?;

        let inst_type = if normalized_inst.ends_with(HYPERLIQUID_PERP_SUFFIX) {
            InstrumentType::Perpetual
        } else {
            InstrumentType::Spot
        };

        hyperliquid_index_to_asset_id(inst_type, index)
    }
}
