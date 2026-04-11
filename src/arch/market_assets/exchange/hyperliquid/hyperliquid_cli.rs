use reqwest::Client;
use serde_json::{Value, json};
use simd_json::from_slice;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tracing::error;

use crate::arch::{
    market_assets::{
        api_data::{
            account_data::{BalanceData, OrderAckData, PositionData},
            price_data::TickerData,
            utils_data::InstrumentInfo,
        },
        api_general::{OrderParams, get_mills_timestamp, value_to_f64},
        base_data::{InstrumentType, MarginMode},
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
        all_mids::RestAllMidsHyperliquid,
        asset_ctxs::{RestMetaAndAssetCtxsHyperliquid, RestSpotMetaAndAssetCtxsHyperliquid},
        clearinghouse_state::RestClearinghouseStateHyperliquid,
        meta::RestMetaHyperliquid,
        spot_clearinghouse_state::RestSpotClearinghouseStateHyperliquid,
        spot_meta::RestSpotMetaHyperliquid,
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
        match channel {
            WsChannel::Trades(Some(TradesParam::AggTrades))
            | WsChannel::Trades(Some(TradesParam::AllTrades))
            | WsChannel::Trades(None) => Ok(HYPERLIQUID_WS.into()),
            _ => Err(InfraError::Unimplemented),
        }
    }

    async fn get_private_connect_msg(&self, channel: &WsChannel) -> InfraResult<String> {
        match channel {
            WsChannel::AccountOrders | WsChannel::AccountPositions => Ok(HYPERLIQUID_WS.into()),
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

    fn owner_address(&self) -> InfraResult<&str> {
        self.auth
            .as_ref()
            .ok_or(InfraError::ApiCliNotInitialized)
            .map(|auth| auth.owner_address.as_str())
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

    fn inst_to_asset_id(&self, inst: &str) -> InfraResult<u32> {
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
            asset: self.inst_to_asset_id(inst)?,
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
            .send_signed_exchange_action::<Value, _>(&self.client, &action, get_mills_timestamp())
            .await?;

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

    async fn _get_tickers(
        &self,
        insts: Option<&[String]>,
        inst_type: Option<InstrumentType>,
    ) -> InfraResult<Vec<TickerData>> {
        match inst_type {
            Some(InstrumentType::Perpetual) => self._get_perp_tickers(insts).await,
            Some(InstrumentType::Spot) => self._get_spot_tickers(insts).await,
            Some(InstrumentType::Unknown) => {
                Err(InfraError::ApiCliError("Unknown instrument type".into()))
            },
            Some(other) => Err(InfraError::ApiCliError(format!(
                "Hyperliquid get_tickers does not support {:?}",
                other
            ))),
            None => {
                let mut tickers = self._get_perp_tickers(None).await?;
                tickers.extend(self._get_spot_tickers(None).await?);

                if let Some(insts) = normalize_inst_filters(insts) {
                    tickers.retain(|ticker| insts.contains(&ticker.inst));
                }

                Ok(tickers)
            },
        }
    }

    async fn _get_perp_tickers(&self, insts: Option<&[String]>) -> InfraResult<Vec<TickerData>> {
        let mids: RestAllMidsHyperliquid = self.post_info(&json!({ "type": "allMids" })).await?;
        let inst_infos = self._get_instrument_info(InstrumentType::Perpetual).await?;
        let normalized_insts = normalize_inst_filters(insts);

        let tickers = inst_infos
            .into_iter()
            .filter(|inst_info| match &normalized_insts {
                Some(insts) => insts.contains(&inst_info.inst),
                None => true,
            })
            .filter_map(|inst_info| {
                let coin = hyperliquid_cli_inst_to_raw_trade_coin(&inst_info.inst)?;
                let price = mids.0.get(&coin)?.parse::<f64>().ok()?;

                Some(TickerData {
                    timestamp: 0,
                    inst: inst_info.inst,
                    inst_type: InstrumentType::Perpetual,
                    price,
                })
            })
            .collect();

        Ok(tickers)
    }

    async fn _get_spot_tickers(&self, insts: Option<&[String]>) -> InfraResult<Vec<TickerData>> {
        let res: RestSpotMetaAndAssetCtxsHyperliquid = self
            .post_info(&json!({ "type": "spotMetaAndAssetCtxs" }))
            .await?;
        let inst_infos = res.0.into_instrument_info();
        let normalized_insts = normalize_inst_filters(insts);

        let tickers = inst_infos
            .into_iter()
            .zip(res.1.into_iter())
            .filter(|(inst_info, _)| match &normalized_insts {
                Some(insts) => insts.contains(&inst_info.inst),
                None => true,
            })
            .map(|(inst_info, ctx)| {
                let price = match value_to_f64(&ctx.midPx) {
                    0.0 => value_to_f64(&ctx.markPx),
                    mid => mid,
                };

                TickerData {
                    timestamp: 0,
                    inst: inst_info.inst,
                    inst_type: InstrumentType::Spot,
                    price,
                }
            })
            .collect();

        Ok(tickers)
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

    async fn _get_balance(&self, assets: Option<&[String]>) -> InfraResult<Vec<BalanceData>> {
        let user = self.owner_address()?;
        let res: RestSpotClearinghouseStateHyperliquid = self
            .post_info(&json!({
                "type": "spotClearinghouseState",
                "user": user,
            }))
            .await?;
        let normalized_assets = normalize_asset_filters(assets);

        let balances = res
            .balances
            .into_iter()
            .map(BalanceData::from)
            .filter(|balance| match &normalized_assets {
                Some(assets) => assets.contains(&balance.asset),
                None => true,
            })
            .collect();

        Ok(balances)
    }

    async fn _get_positions(&self, insts: Option<&[String]>) -> InfraResult<Vec<PositionData>> {
        let user = self.owner_address()?;
        let state: RestClearinghouseStateHyperliquid = self
            .post_info(&json!({
                "type": "clearinghouseState",
                "user": user,
            }))
            .await?;
        let ctxs: RestMetaAndAssetCtxsHyperliquid = self
            .post_info(&json!({ "type": "metaAndAssetCtxs" }))
            .await?;
        let normalized_insts = normalize_inst_filters(insts);

        let mark_px_by_coin: HashMap<String, f64> = ctxs
            .0
            .universe
            .into_iter()
            .zip(ctxs.1.into_iter())
            .map(|(meta, ctx)| {
                let mark_price = match value_to_f64(&ctx.markPx) {
                    0.0 => value_to_f64(&ctx.midPx),
                    mark => mark,
                };
                (meta.name, mark_price)
            })
            .collect();

        let positions = state
            .assetPositions
            .into_iter()
            .map(|position| {
                let mark_price = mark_px_by_coin
                    .get(&position.position.coin)
                    .copied()
                    .unwrap_or_default();
                position.into_position_data(mark_price)
            })
            .filter(|position| match &normalized_insts {
                Some(insts) => insts.contains(&position.inst),
                None => true,
            })
            .collect();

        Ok(positions)
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
        let user = self.owner_address()?;

        match channel {
            WsChannel::AccountOrders => Ok(ws_subscribe_msg_hyperliquid_user("orderUpdates", user)),
            WsChannel::AccountPositions => Ok(ws_subscribe_msg_hyperliquid_user(
                "clearinghouseState",
                user,
            )),
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
}

fn normalize_inst_filters(insts: Option<&[String]>) -> Option<HashSet<String>> {
    insts.map(|insts| {
        insts
            .iter()
            .map(|inst| normalize_hyperliquid_cli_inst(inst))
            .collect()
    })
}

fn normalize_asset_filters(assets: Option<&[String]>) -> Option<HashSet<String>> {
    assets.map(|assets| {
        assets
            .iter()
            .map(|asset| hyperliquid_symbol_to_cli_symbol(asset))
            .collect()
    })
}
