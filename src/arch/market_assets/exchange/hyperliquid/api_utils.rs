use serde::Serialize;

use crate::arch::market_assets::{
    api_general::OrderParams,
    base_data::{OrderSide, OrderType, TimeInForce},
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

const HYPERLIQUID_MARKET_BUY_MAX_PX: &str = "1000000000000";
const HYPERLIQUID_MARKET_SELL_MIN_PX: &str = "0";

#[derive(Clone, Debug, Serialize)]
pub struct HyperliquidOrderAction {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub orders: Vec<HyperliquidOrderRequest>,
    pub grouping: &'static str,
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
        OrderType::Market => (
            if is_buy {
                HYPERLIQUID_MARKET_BUY_MAX_PX.to_string()
            } else {
                HYPERLIQUID_MARKET_SELL_MIN_PX.to_string()
            },
            "Ioc",
        ),
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
