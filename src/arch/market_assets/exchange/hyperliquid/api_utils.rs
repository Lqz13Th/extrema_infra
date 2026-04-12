use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;

use crate::arch::market_assets::{
    api_general::OrderParams,
    base_data::{InstrumentType, OrderSide, OrderType, TimeInForce},
};
use crate::errors::{InfraError, InfraResult};

pub const HYPERLIQUID_QUOTE: &str = "USDC";
pub const HYPERLIQUID_PERP_SUFFIX: &str = "_USDC_PERP";
pub const HYPERLIQUID_SPOT_ASSET_OFFSET: u32 = 10_000;
pub const HYPERLIQUID_FUNDING_INTERVAL_HOURS: u64 = 1;
const HYPERLIQUID_FUNDING_INTERVAL_MS: u64 = HYPERLIQUID_FUNDING_INTERVAL_HOURS * 60 * 60 * 1000;
const HYPERLIQUID_KILO_PREFIX: &str = "k";
const CLI_KILO_PREFIX: &str = "1000";

pub fn hyperliquid_perp_asset_id(index: usize) -> String {
    index.to_string()
}

pub fn hyperliquid_spot_asset_id(index: u32) -> String {
    (HYPERLIQUID_SPOT_ASSET_OFFSET + index).to_string()
}

pub fn hyperliquid_index_to_asset_id(inst_type: InstrumentType, index: u32) -> InfraResult<u32> {
    match inst_type {
        InstrumentType::Perpetual => Ok(index),
        InstrumentType::Spot => Ok(HYPERLIQUID_SPOT_ASSET_OFFSET + index),
        _ => Err(InfraError::ApiCliError(format!(
            "Unsupported Hyperliquid instrument type for index-to-asset conversion: {:?}",
            inst_type
        ))),
    }
}

pub fn hyperliquid_asset_id_to_index(
    inst_type: InstrumentType,
    asset_id: &str,
) -> InfraResult<u32> {
    let asset_id = asset_id.parse::<u32>().map_err(|_| {
        InfraError::ApiCliError(format!("Invalid Hyperliquid asset id: {}", asset_id))
    })?;

    match inst_type {
        InstrumentType::Perpetual => Ok(asset_id),
        InstrumentType::Spot => asset_id
            .checked_sub(HYPERLIQUID_SPOT_ASSET_OFFSET)
            .ok_or_else(|| {
                InfraError::ApiCliError(format!(
                    "Hyperliquid spot asset id below offset {}: {}",
                    HYPERLIQUID_SPOT_ASSET_OFFSET, asset_id
                ))
            }),
        _ => Err(InfraError::ApiCliError(format!(
            "Unsupported Hyperliquid instrument type for asset-id to index conversion: {:?}",
            inst_type
        ))),
    }
}

pub fn hyperliquid_symbol_to_cli_symbol(symbol: &str) -> String {
    if let Some(rest) = symbol.strip_prefix(HYPERLIQUID_KILO_PREFIX)
        && is_hyperliquid_kilo_symbol(rest)
    {
        return format!("{}{}", CLI_KILO_PREFIX, rest);
    }

    symbol.to_string()
}

fn hyperliquid_cli_symbol_to_raw_symbol(symbol: &str) -> String {
    if let Some(rest) = symbol.strip_prefix(CLI_KILO_PREFIX)
        && is_hyperliquid_kilo_symbol(rest)
    {
        return format!("{}{}", HYPERLIQUID_KILO_PREFIX, rest);
    }

    symbol.to_string()
}

pub(crate) fn normalize_hyperliquid_cli_inst(inst: &str) -> String {
    if let Some(base) = inst.strip_suffix(HYPERLIQUID_PERP_SUFFIX) {
        return format!(
            "{}{}",
            hyperliquid_symbol_to_cli_symbol(base),
            HYPERLIQUID_PERP_SUFFIX
        );
    }

    if let Some((base, quote)) = inst.split_once('_') {
        return format!(
            "{}_{}",
            hyperliquid_symbol_to_cli_symbol(base),
            hyperliquid_symbol_to_cli_symbol(quote)
        );
    }

    hyperliquid_symbol_to_cli_symbol(inst)
}

pub fn hyperliquid_cli_inst_to_raw_perp_coin(inst: &str) -> InfraResult<String> {
    let normalized_inst = normalize_hyperliquid_cli_inst(inst);

    normalized_inst
        .strip_suffix(HYPERLIQUID_PERP_SUFFIX)
        .map(hyperliquid_cli_symbol_to_raw_symbol)
        .ok_or_else(|| {
            InfraError::ApiCliError(format!(
                "Hyperliquid funding supports perpetual instruments only: {}",
                inst
            ))
        })
}

pub(crate) fn hyperliquid_cli_inst_to_raw_trade_coin(inst: &str) -> Option<String> {
    if let Ok(coin) = hyperliquid_cli_inst_to_raw_perp_coin(inst) {
        return Some(coin);
    }

    if inst == "PURR_USDC" {
        return Some("PURR/USDC".into());
    }

    None
}

pub fn hyperliquid_perp_to_cli(symbol: &str) -> String {
    format!(
        "{}{}",
        hyperliquid_symbol_to_cli_symbol(symbol),
        HYPERLIQUID_PERP_SUFFIX
    )
}

pub fn hyperliquid_spot_to_cli(symbol: &str, base: &str, quote: &str) -> String {
    if let Some((base, quote)) = symbol.split_once('/') {
        return format!(
            "{}_{}",
            hyperliquid_symbol_to_cli_symbol(base),
            hyperliquid_symbol_to_cli_symbol(quote)
        );
    }

    format!(
        "{}_{}",
        hyperliquid_symbol_to_cli_symbol(base),
        hyperliquid_symbol_to_cli_symbol(quote)
    )
}

pub fn hyperliquid_inst_to_cli(coin: &str) -> String {
    if coin.starts_with('@') {
        return coin.to_string();
    }

    if let Some((base, quote)) = coin.split_once('/') {
        return format!(
            "{}_{}",
            hyperliquid_symbol_to_cli_symbol(base),
            hyperliquid_symbol_to_cli_symbol(quote)
        );
    }

    hyperliquid_perp_to_cli(coin)
}

pub fn hyperliquid_funding_interval_hours() -> u64 {
    HYPERLIQUID_FUNDING_INTERVAL_HOURS
}

pub fn hyperliquid_funding_interval_sec() -> f64 {
    HYPERLIQUID_FUNDING_INTERVAL_MS as f64 / 1000.0
}

pub fn hyperliquid_next_funding_time_ms(now_ms: u64) -> u64 {
    (now_ms / HYPERLIQUID_FUNDING_INTERVAL_MS)
        .saturating_add(1)
        .saturating_mul(HYPERLIQUID_FUNDING_INTERVAL_MS)
}

pub fn normalize_inst_filters(insts: Option<&[String]>) -> Option<HashSet<String>> {
    insts.map(|insts| {
        insts
            .iter()
            .map(|inst| normalize_hyperliquid_cli_inst(inst))
            .collect()
    })
}

pub fn normalize_funding_inst_filter(inst: Option<&str>) -> InfraResult<Option<String>> {
    match inst {
        Some(inst) => {
            let normalized = normalize_hyperliquid_cli_inst(inst);
            if !normalized.ends_with(HYPERLIQUID_PERP_SUFFIX) {
                return Err(InfraError::ApiCliError(format!(
                    "Hyperliquid funding supports perpetual instruments only: {}",
                    inst
                )));
            }

            Ok(Some(normalized))
        },
        None => Ok(None),
    }
}

pub fn normalize_asset_filters(assets: Option<&[String]>) -> Option<HashSet<String>> {
    assets.map(|assets| {
        assets
            .iter()
            .map(|asset| hyperliquid_symbol_to_cli_symbol(asset))
            .collect()
    })
}

pub fn ws_subscribe_msg_hyperliquid_trades(coin: &str) -> String {
    json!({
        "method": "subscribe",
        "subscription": {
            "type": "trades",
            "coin": coin.to_string(),
        }
    })
    .to_string()
}

pub fn ws_subscribe_msg_hyperliquid_user(subscription_type: &str, user: &str) -> String {
    json!({
        "method": "subscribe",
        "subscription": {
            "type": subscription_type,
            "user": user,
        }
    })
    .to_string()
}

#[derive(Clone, Debug, Serialize)]
pub struct HyperliquidOrderAction {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub orders: Vec<HyperliquidOrderRequest>,
    pub grouping: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct HyperliquidUpdateLeverageAction {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub asset: u32,
    #[serde(rename = "isCross")]
    pub is_cross: bool,
    pub leverage: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct HyperliquidOrderRequest {
    #[serde(rename = "a")]
    asset: u32,
    #[serde(rename = "b")]
    is_buy: bool,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "s")]
    size: String,
    #[serde(rename = "r")]
    reduce_only: bool,
    #[serde(rename = "t")]
    order_type: HyperliquidOrderTypeRequest,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
enum HyperliquidOrderTypeRequest {
    Limit { limit: HyperliquidLimitOrderRequest },
}

#[derive(Clone, Debug, Serialize)]
struct HyperliquidLimitOrderRequest {
    tif: &'static str,
}

pub fn hyperliquid_order_from_params(
    order_params: OrderParams,
) -> InfraResult<HyperliquidOrderRequest> {
    let asset = order_params.inst.parse::<u32>().map_err(|_| {
        InfraError::ApiCliError(format!(
            "Invalid Hyperliquid asset id in OrderParams.inst: {}",
            order_params.inst
        ))
    })?;

    let is_buy = match order_params.side {
        OrderSide::BUY => true,
        OrderSide::SELL => false,
        OrderSide::Unknown => true,
    };

    let reduce_only = order_params.reduce_only.unwrap_or(false);
    let size = normalize_hyperliquid_num_str(&order_params.size);

    let (price, tif) = match order_params.order_type {
        OrderType::Market => {
            return Err(InfraError::ApiCliError(
                "Hyperliquid market orders are disabled in this client; use IOC/FOK/limit with an explicit price".into(),
            ));
        },
        OrderType::PostOnly => (
            normalize_hyperliquid_num_str(order_params.price.as_deref().ok_or(
                InfraError::ApiCliError("Hyperliquid post-only order requires price".into()),
            )?),
            "Alo",
        ),
        OrderType::Ioc | OrderType::Fok => (
            normalize_hyperliquid_num_str(order_params.price.as_deref().ok_or(
                InfraError::ApiCliError("Hyperliquid IOC/FOK order requires price".into()),
            )?),
            "Ioc",
        ),
        OrderType::Limit | OrderType::Unknown => (
            normalize_hyperliquid_num_str(order_params.price.as_deref().ok_or(
                InfraError::ApiCliError("Hyperliquid limit order requires price".into()),
            )?),
            match order_params.time_in_force.unwrap_or(TimeInForce::GTC) {
                TimeInForce::IOC | TimeInForce::FOK => "Ioc",
                TimeInForce::GTC | TimeInForce::GTD | TimeInForce::Unknown => "Gtc",
            },
        ),
    };

    Ok(HyperliquidOrderRequest {
        asset,
        is_buy,
        price,
        size,
        reduce_only,
        order_type: HyperliquidOrderTypeRequest::Limit {
            limit: HyperliquidLimitOrderRequest { tif },
        },
    })
}

pub fn normalize_hyperliquid_num_str(input: &str) -> String {
    let trimmed = input.trim();
    if let Some((whole, frac)) = trimmed.split_once('.') {
        let frac = frac.trim_end_matches('0');
        if frac.is_empty() {
            if whole.is_empty() {
                "0".into()
            } else {
                whole.to_string()
            }
        } else {
            format!("{}.{}", whole, frac)
        }
    } else if trimmed.is_empty() {
        "0".into()
    } else {
        trimmed.to_string()
    }
}

fn is_hyperliquid_kilo_symbol(symbol: &str) -> bool {
    !symbol.is_empty()
        && symbol
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}
