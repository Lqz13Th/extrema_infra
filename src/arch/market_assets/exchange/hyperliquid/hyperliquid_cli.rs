use reqwest::Client;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, warn};

use crate::arch::{
    market_assets::{
        api_data::{
            account_data::{BalanceData, HistoOrderData, OrderAckData, PositionData},
            price_data::{CandleData, OrderBookData, TickerData},
            utils_data::{FundingRateData, FundingRateInfo, InstrumentInfo},
        },
        api_general::{
            OrderParams, get_micros_timestamp, get_mills_timestamp, parse_json_response,
        },
        base_data::{InstrumentType, MarginMode},
    },
    task_execution::task_ws::{CandleParam, LobParam, TradesParam, WsChannel},
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
        candle::RestCandleHyperliquid, clearinghouse_state::RestClearinghouseStateHyperliquid,
        funding_history::RestFundingHistoryHyperliquid, meta::RestMetaHyperliquid,
        order_status::RestOrderStatusHyperliquid, orderbook::RestOrderBookHyperliquid,
        perp_dexs::RestPerpDexHyperliquid,
        spot_clearinghouse_state::RestSpotClearinghouseStateHyperliquid,
        spot_meta::RestSpotMetaHyperliquid, trade_order::RestOrderAckHyperliquid,
    },
};

#[derive(Clone, Debug)]
pub struct HyperliquidCli {
    pub client: Arc<Client>,
    pub auth: Option<HyperliquidAuth>,
    pub market_cache: HyperliquidMarketCache,
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

    async fn get_candles(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        interval: CandleParam,
        limit: Option<u32>,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
    ) -> InfraResult<Vec<CandleData>> {
        self._get_candles(inst, inst_type, interval, limit, start_time_us, end_time_us)
            .await
    }

    async fn get_orderbook(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        depth: usize,
    ) -> InfraResult<OrderBookData> {
        self._get_orderbook(inst, inst_type, depth).await
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
    const DEFAULT_CANDLE_LIMIT: u32 = 7;

    pub fn new(shared_client: Arc<Client>) -> Self {
        Self {
            client: shared_client,
            auth: None,
            market_cache: HyperliquidMarketCache::default(),
        }
    }

    pub fn set_perp_dex(&mut self, dex: Option<String>) {
        let normalized_dex = dex.and_then(|dex| {
            let dex = dex.trim().to_string();
            (!dex.is_empty()).then_some(dex)
        });

        self.market_cache.set_perp_dex(normalized_dex);
    }

    pub async fn init_inst_index_map(&mut self) -> InfraResult<()> {
        if self.market_cache.perp_dex.is_some() && self.market_cache.perp_dex_index.is_none() {
            self.init_perp_dex_index().await?;
        }

        let mut inst_index_map = HashMap::new();

        let (perp_infos, perp_quote) = self._get_perp_instrument_info_with_quote().await?;
        self.market_cache.perp_quote = Some(perp_quote);

        for inst_info in perp_infos {
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

        self.market_cache.inst_index_map = inst_index_map;

        Ok(())
    }

    pub async fn init_perp_dex_index(&mut self) -> InfraResult<()> {
        self.market_cache.perp_dex_index = self._resolve_perp_dex_index().await?;
        Ok(())
    }

    pub async fn get_funding_rate_live(
        &self,
        inst: Option<&str>,
    ) -> InfraResult<Vec<FundingRateData>> {
        let target_inst = normalize_funding_inst_filter(inst)?;

        let ctxs = self._get_meta_and_asset_ctxs().await?;
        let quote = self._perp_quote_from_meta(&ctxs.0).await?;
        let data = ctxs
            .into_funding_rate_data(&quote)?
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

        let ctxs = self._get_meta_and_asset_ctxs().await?;
        let quote = self._perp_quote_from_meta(&ctxs.0).await?;
        let data = ctxs
            .into_funding_rate_info(&quote)?
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

        let normalized_inst = normalize_hyperliquid_cli_inst(inst);
        self._ensure_perp_quote_matches(&normalized_inst)?;
        let quote = hyperliquid_cli_perp_quote(&normalized_inst)?;
        let coin = self._inst_to_raw_perp_coin(&normalized_inst)?;
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
            .map(|entry| entry.into_funding_rate_data(&quote))
            .collect();

        Ok(data)
    }

    async fn _get_candles(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        interval: CandleParam,
        limit: Option<u32>,
        start_time_us: Option<u64>,
        end_time_us: Option<u64>,
    ) -> InfraResult<Vec<CandleData>> {
        let coin = self._inst_to_trade_coin_for_type(inst, inst_type)?;
        let limit = limit.unwrap_or(Self::DEFAULT_CANDLE_LIMIT);
        let interval_ms = candle_interval_millis(&interval)?;
        let end_time = end_time_us
            .map(|end| end / 1_000)
            .unwrap_or_else(get_mills_timestamp);
        let start_time = start_time_us
            .map(|start| start / 1_000)
            .unwrap_or_else(|| end_time.saturating_sub(limit as u64 * interval_ms));

        let body = json!({
            "type": "candleSnapshot",
            "req": {
                "coin": coin,
                "interval": interval.as_str(),
                "startTime": start_time,
                "endTime": end_time,
            },
        });
        let res: RestResHyperliquid<RestCandleHyperliquid> = self._post_info_raw(&body).await?;

        let mut data: Vec<CandleData> = res
            .into_vec()?
            .into_iter()
            .map(|entry| entry.into_candle_data(inst))
            .collect();
        data.sort_by_key(|candle| candle.timestamp);

        if data.len() > limit as usize {
            data.drain(..data.len() - limit as usize);
        }

        Ok(data)
    }

    async fn _get_orderbook(
        &self,
        inst: &str,
        inst_type: InstrumentType,
        depth: usize,
    ) -> InfraResult<OrderBookData> {
        let coin = self._inst_to_trade_coin_for_type(inst, inst_type)?;
        let body = json!({
            "type": "l2Book",
            "coin": coin,
        });
        let res: RestResHyperliquid<RestOrderBookHyperliquid> = self._post_info_raw(&body).await?;
        let book = res
            .into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid l2Book data returned".into(),
            ))?;

        Ok(book.into_orderbook_data(inst, depth))
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

        let normalized_inst = normalize_hyperliquid_cli_inst(inst);
        if !is_hyperliquid_cli_perp_inst(&normalized_inst) {
            return Err(InfraError::ApiCliError(
                "Hyperliquid set_leverage supports perpetual instruments only".into(),
            ));
        }
        self._ensure_perp_quote_matches(&normalized_inst)?;

        let action = HyperliquidUpdateLeverageAction {
            kind: "updateLeverage",
            asset: self._inst_to_asset_id(&normalized_inst)?,
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
                let (data, _) = self._get_perp_instrument_info_with_quote().await?;
                Ok(data)
            },
            InstrumentType::Spot => Ok(self._get_spot_meta().await?.into_instrument_info()),
            _ => Err(InfraError::ApiCliError(
                "Hyperliquid get_instrument_info currently supports Spot and Perpetual only".into(),
            )),
        }
    }

    async fn _get_perp_instrument_info_with_quote(
        &self,
    ) -> InfraResult<(Vec<InstrumentInfo>, String)> {
        let meta = self._get_perp_meta().await?;
        let quote = self._perp_quote_from_meta(&meta).await?;
        Ok((meta.into_instrument_info(&quote), quote))
    }

    async fn _get_perp_meta(&self) -> InfraResult<RestMetaHyperliquid> {
        let body = json!({ "type": "meta", "dex": self._perp_dex() });
        let res: RestResHyperliquid<RestMetaHyperliquid> = self._post_info_raw(&body).await?;

        res.into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid perpetual instrument info returned".into(),
            ))
    }

    async fn _get_spot_meta(&self) -> InfraResult<RestSpotMetaHyperliquid> {
        let body = json!({ "type": "spotMeta" });
        let res: RestResHyperliquid<RestSpotMetaHyperliquid> = self._post_info_raw(&body).await?;

        res.into_vec()?
            .into_iter()
            .next()
            .ok_or(InfraError::ApiCliError(
                "No Hyperliquid spot instrument info returned".into(),
            ))
    }

    async fn _perp_quote_from_meta(&self, meta: &RestMetaHyperliquid) -> InfraResult<String> {
        let Some(collateral_token) = meta.collateral_token else {
            return Ok(HYPERLIQUID_QUOTE.to_string());
        };

        let spot_meta = self._get_spot_meta().await?;
        spot_meta
            .token_name(collateral_token)
            .map(hyperliquid_symbol_to_cli_symbol)
            .ok_or_else(|| {
                InfraError::ApiCliError(format!(
                    "Hyperliquid collateral token not found in spotMeta: {}",
                    collateral_token
                ))
            })
    }

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        match inst_type.unwrap_or(InstrumentType::Perpetual) {
            InstrumentType::Perpetual => {
                let quote = self._perp_quote_for_conversion()?;
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
                    .into_perp_ticker_data(get_micros_timestamp(), quote)
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

                let spot_inst_by_coin = self._get_spot_meta().await?.into_spot_inst_by_coin();
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
        let builder = hyperliquid_builder_fee_from_extra(&order_params.extra)?;

        let action = HyperliquidOrderAction {
            kind: "order",
            orders: vec![hyperliquid_order_from_params(order_params)?],
            grouping: HYPERLIQUID_GROUPING_NA,
            builder,
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
        let ctxs = self._get_meta_and_asset_ctxs().await?;
        let quote = self._perp_quote_from_meta(&ctxs.0).await?;
        let mark_px_by_coin = ctxs.into_perp_mark_px_by_coin()?;

        let positions = data
            .into_position_data(&mark_px_by_coin, &quote)
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
        let perp_quote = if is_hyperliquid_cli_perp_inst(&normalized_inst) {
            Some(hyperliquid_cli_perp_quote(&normalized_inst)?)
        } else {
            None
        };

        let res: RestResHyperliquid<RestOrderStatusHyperliquid> =
            self._post_info_raw(&body).await?;

        let mut data: Vec<HistoOrderData> = res
            .into_vec()?
            .into_iter()
            .map(|order| order.into_histo_order_data(perp_quote.as_deref()))
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
            | WsChannel::Trades(None)
            | WsChannel::Lob(_) => Ok(HYPERLIQUID_WS.into()),
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
            WsChannel::Lob(lob_param) => self._ws_subscribe_lob(lob_param, insts),
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

        if insts.len() > 1 {
            warn!(
                "Hyperliquid trades ws supports one instrument per subscription message; got {} instruments: {:?}",
                insts.len(),
                insts
            );
        }

        let inst = insts.first().expect("checked non-empty insts");
        let coin = self._inst_to_trade_coin(inst)?;
        Ok(json!({
            "method": "subscribe",
            "subscription": {
                "type": "trades",
                "coin": coin.to_string(),
            }
        })
        .to_string())
    }

    fn _ws_subscribe_lob(
        &self,
        lob_param: &Option<LobParam>,
        insts: Option<&[String]>,
    ) -> InfraResult<String> {
        let insts = insts.ok_or_else(|| {
            InfraError::ApiCliError("Hyperliquid lob ws requires at least one instrument".into())
        })?;

        if insts.is_empty() {
            return Err(InfraError::ApiCliError(
                "Hyperliquid lob ws requires at least one instrument".into(),
            ));
        }

        if insts.len() > 1 {
            warn!(
                "Hyperliquid lob ws supports one instrument per subscription message; got {} instruments: {:?}",
                insts.len(),
                insts
            );
        }

        let subscription_type = hyperliquid_lob_subscription_type(lob_param)?;
        let inst = insts.first().expect("checked non-empty insts");
        let coin = self._inst_to_trade_coin(inst)?;
        Ok(json!({
            "method": "subscribe",
            "subscription": {
                "type": subscription_type,
                "coin": coin.to_string(),
            }
        })
        .to_string())
    }

    fn _inst_to_trade_coin(&self, inst: &str) -> InfraResult<String> {
        let normalized_inst = normalize_hyperliquid_cli_inst(inst);
        if is_hyperliquid_cli_perp_inst(&normalized_inst) {
            self._ensure_perp_quote_matches(&normalized_inst)?;
            return self._inst_to_raw_perp_coin(&normalized_inst);
        }

        if let Some(coin) = hyperliquid_cli_inst_to_raw_trade_coin(inst) {
            return Ok(coin);
        }

        let index = self
            .market_cache
            .inst_index_map
            .get(&normalized_inst)
            .copied()
            .ok_or_else(|| {
                if self.market_cache.inst_index_map.is_empty() {
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

    fn _inst_to_trade_coin_for_type(
        &self,
        inst: &str,
        inst_type: InstrumentType,
    ) -> InfraResult<String> {
        let normalized_inst = normalize_hyperliquid_cli_inst(inst);
        match inst_type {
            InstrumentType::Perpetual => {
                if !is_hyperliquid_cli_perp_inst(&normalized_inst) {
                    return Err(InfraError::ApiCliError(format!(
                        "Hyperliquid perpetual instrument expected, got {}",
                        inst
                    )));
                }
                self._ensure_perp_quote_matches(&normalized_inst)?;
                self._inst_to_raw_perp_coin(&normalized_inst)
            },
            InstrumentType::Spot => {
                if is_hyperliquid_cli_perp_inst(&normalized_inst) {
                    return Err(InfraError::ApiCliError(format!(
                        "Hyperliquid spot instrument expected, got {}",
                        inst
                    )));
                }
                self._inst_to_spot_coin(&normalized_inst, inst)
            },
            _ => Err(InfraError::ApiCliError(format!(
                "Hyperliquid market data supports Spot and Perpetual only, got {:?}",
                inst_type
            ))),
        }
    }

    fn _inst_to_spot_coin(
        &self,
        normalized_inst: &str,
        original_inst: &str,
    ) -> InfraResult<String> {
        if let Some(coin) = hyperliquid_cli_inst_to_raw_trade_coin(original_inst) {
            return Ok(coin);
        }

        let index = self
            .market_cache
            .inst_index_map
            .get(normalized_inst)
            .copied()
            .ok_or_else(|| {
                if self.market_cache.inst_index_map.is_empty() {
                    InfraError::ApiCliError(
                        "Hyperliquid inst_index_map is empty, call init_inst_index_map() first"
                            .into(),
                    )
                } else {
                    InfraError::ApiCliError(format!(
                        "Hyperliquid spot inst not found in inst_index_map: {}",
                        original_inst
                    ))
                }
            })?;

        Ok(format!("@{}", index))
    }

    fn _inst_to_raw_perp_coin(&self, inst: &str) -> InfraResult<String> {
        let base = hyperliquid_cli_inst_to_raw_perp_coin(inst)?;
        match self.market_cache.perp_dex.as_deref() {
            Some(dex) => Ok(format!("{dex}:{base}")),
            None => Ok(base),
        }
    }

    fn _owner_address(&self) -> InfraResult<&str> {
        self.auth
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)
            .map(|auth| auth.owner_address.as_str())
    }

    fn _perp_dex(&self) -> &str {
        self.market_cache.perp_dex()
    }

    fn _perp_quote_for_conversion(&self) -> InfraResult<&str> {
        match (
            self.market_cache.perp_dex.as_deref(),
            self.market_cache.perp_quote.as_deref(),
        ) {
            (Some(_), Some(quote)) => Ok(quote),
            (Some(dex), None) => Err(InfraError::ApiCliError(format!(
                "Hyperliquid perp quote is not initialized for dex {}, call init_inst_index_map() first",
                dex
            ))),
            (None, Some(quote)) => Ok(quote),
            (None, None) => Ok(HYPERLIQUID_QUOTE),
        }
    }

    fn _ensure_perp_quote_matches(&self, inst: &str) -> InfraResult<()> {
        let inst_quote = hyperliquid_cli_perp_quote(inst)?;
        let expected_quote = self._perp_quote_for_conversion()?;
        if inst_quote != expected_quote {
            return Err(InfraError::ApiCliError(format!(
                "Hyperliquid perp quote mismatch for {}: expected {} for dex {}, got {}",
                inst,
                expected_quote,
                self._perp_dex(),
                inst_quote
            )));
        }

        Ok(())
    }

    async fn _resolve_perp_dex_index(&self) -> InfraResult<Option<u32>> {
        let Some(dex) = self.market_cache.perp_dex.as_deref() else {
            return Ok(None);
        };

        let res: RestResHyperliquid<Option<RestPerpDexHyperliquid>> =
            self._post_info_raw(&json!({ "type": "perpDexs" })).await?;

        for (index, perp_dex) in res.into_vec()?.into_iter().enumerate() {
            let Some(perp_dex) = perp_dex else {
                continue;
            };

            if perp_dex.name == dex {
                return u32::try_from(index).map(Some).map_err(|_| {
                    InfraError::ApiCliError(format!(
                        "Hyperliquid perp dex index overflow for dex {}: {}",
                        dex, index
                    ))
                });
            }
        }

        Err(InfraError::ApiCliError(format!(
            "Hyperliquid perp dex not found in perpDexs: {}",
            dex
        )))
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
            .market_cache
            .inst_index_map
            .get(&normalized_inst)
            .copied()
            .ok_or_else(|| {
                if self.market_cache.inst_index_map.is_empty() {
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

        if is_hyperliquid_cli_perp_inst(&normalized_inst) {
            self._ensure_perp_quote_matches(&normalized_inst)?;
            let perp_dex_index = match self.market_cache.perp_dex.as_deref() {
                Some(dex) => Some(self.market_cache.perp_dex_index.ok_or_else(|| {
                    InfraError::ApiCliError(format!(
                        "Hyperliquid perp dex index is not initialized for dex {}, call init_inst_index_map() first",
                        dex
                    ))
                })?),
                None => None,
            };

            hyperliquid_perp_asset_id_for_dex(index, perp_dex_index)
        } else {
            hyperliquid_index_to_asset_id(InstrumentType::Spot, index)
        }
    }
}

fn candle_interval_millis(interval: &CandleParam) -> InfraResult<u64> {
    match interval {
        CandleParam::OneSecond => Ok(1_000),
        CandleParam::OneMinute => Ok(60_000),
        CandleParam::FiveMinutes => Ok(5 * 60_000),
        CandleParam::FifteenMinutes => Ok(15 * 60_000),
        CandleParam::OneHour => Ok(60 * 60_000),
        CandleParam::FourHours => Ok(4 * 60 * 60_000),
        CandleParam::OneDay => Ok(24 * 60 * 60_000),
        CandleParam::OneWeek => Ok(7 * 24 * 60 * 60_000),
        CandleParam::Custom(value) => Err(InfraError::ApiCliError(format!(
            "Hyperliquid candle interval duration is unknown for custom interval: {}",
            value
        ))),
    }
}
