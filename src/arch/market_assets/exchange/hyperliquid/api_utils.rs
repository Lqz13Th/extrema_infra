use serde::Serialize;
use serde_json::json;

use crate::arch::market_assets::{
    api_general::OrderParams,
    base_data::{InstrumentType, OrderSide, OrderType, TimeInForce},
};
use crate::errors::{InfraError, InfraResult};

pub const HYPERLIQUID_QUOTE: &str = "USDC";
pub const HYPERLIQUID_SPOT_ASSET_OFFSET: u32 = 10_000;

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

pub fn hyperliquid_perp_to_cli(symbol: &str) -> String {
    format!("{}_{}_PERP", symbol, HYPERLIQUID_QUOTE)
}

pub fn hyperliquid_spot_to_cli(symbol: &str, base: &str, quote: &str) -> String {
    if symbol.contains('/') {
        symbol.replace('/', "_")
    } else {
        format!("{}_{}", base, quote)
    }
}

pub fn hyperliquid_trade_coin_to_cli(coin: &str) -> String {
    if coin.starts_with('@') {
        coin.to_string()
    } else if coin.contains('/') {
        coin.replace('/', "_")
    } else {
        hyperliquid_perp_to_cli(coin)
    }
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
