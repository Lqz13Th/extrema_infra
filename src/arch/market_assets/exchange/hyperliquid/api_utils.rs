use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::arch::{
    market_assets::{
        api_general::OrderParams,
        base_data::{InstrumentType, OrderSide, OrderType, TimeInForce},
    },
    task_execution::task_ws::{LobFrequency, LobParam},
};
use crate::errors::{InfraError, InfraResult};

pub const HYPERLIQUID_QUOTE: &str = "USDC";
pub const HYPERLIQUID_PERP_TYPE_SUFFIX: &str = "_PERP";
pub const HYPERLIQUID_SPOT_ASSET_OFFSET: u32 = 10_000;
pub const HYPERLIQUID_BUILDER_PERP_ASSET_OFFSET: u32 = 100_000;
pub const HYPERLIQUID_BUILDER_PERP_DEX_STRIDE: u32 = 10_000;
pub const HYPERLIQUID_FUNDING_INTERVAL_HOURS: u64 = 1;
pub const HYPERLIQUID_BUILDER_ADDRESS_EXTRA_KEY: &str = "builder_b";
pub const HYPERLIQUID_BUILDER_FEE_EXTRA_KEY: &str = "builder_f";
pub const HYPERLIQUID_PERP_DEX_SCOPE_PREFIX: &str = "hl_dex:";
const HYPERLIQUID_FUNDING_INTERVAL_MS: u64 = HYPERLIQUID_FUNDING_INTERVAL_HOURS * 60 * 60 * 1000;
const HYPERLIQUID_KILO_PREFIX: &str = "k";
const CLI_KILO_PREFIX: &str = "1000";

#[derive(Clone, Debug, Default)]
pub struct HyperliquidMarketCache {
    pub inst_index_map: HashMap<String, u32>,
    pub perp_dex: Option<String>,
    pub perp_dex_index: Option<u32>,
    pub perp_quote: Option<String>,
}

impl HyperliquidMarketCache {
    pub fn set_perp_dex(&mut self, dex: Option<String>) {
        if self.perp_dex != dex {
            self.inst_index_map.clear();
            self.perp_dex_index = None;
            self.perp_quote = None;
        }

        self.perp_dex = dex;
    }

    pub fn perp_dex(&self) -> &str {
        self.perp_dex.as_deref().unwrap_or("")
    }
}

pub fn hyperliquid_perp_asset_id(index: usize) -> String {
    index.to_string()
}

pub fn hyperliquid_scope_extra_from_dex(dex: &str) -> Option<String> {
    let dex = dex.trim();
    if dex.is_empty() {
        None
    } else {
        Some(format!("{HYPERLIQUID_PERP_DEX_SCOPE_PREFIX}{dex}"))
    }
}

pub fn hyperliquid_dex_from_scope_extra(extra: Option<&str>) -> Option<String> {
    extra?
        .strip_prefix(HYPERLIQUID_PERP_DEX_SCOPE_PREFIX)
        .map(str::trim)
        .filter(|dex| !dex.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn hyperliquid_lob_subscription_type(
    lob_param: &Option<LobParam>,
) -> InfraResult<&'static str> {
    match lob_param {
        None => Ok("l2Book"),
        Some(LobParam::Bbo { frequency }) => match frequency {
            None | Some(LobFrequency::Realtime) => Ok("bbo"),
            Some(freq) => Err(InfraError::ApiCliError(format!(
                "Hyperliquid bbo does not support requested frequency: {:?}",
                freq
            ))),
        },
        Some(LobParam::Snapshot { depth, frequency }) => {
            if depth.is_none() && frequency.is_none() {
                Ok("l2Book")
            } else {
                Err(InfraError::ApiCliError(format!(
                    "Hyperliquid l2Book does not support depth/frequency: depth={:?}, frequency={:?}",
                    depth, frequency
                )))
            }
        },
        Some(LobParam::Incremental { depth, frequency }) => Err(InfraError::ApiCliError(format!(
            "Hyperliquid does not support incremental LOB updates: depth={:?}, frequency={:?}",
            depth, frequency
        ))),
    }
}

pub fn hyperliquid_spot_asset_id(index: u32) -> String {
    (HYPERLIQUID_SPOT_ASSET_OFFSET + index).to_string()
}

pub fn hyperliquid_perp_asset_id_for_dex(
    index_in_meta: u32,
    perp_dex_index: Option<u32>,
) -> InfraResult<u32> {
    let Some(perp_dex_index) = perp_dex_index else {
        return Ok(index_in_meta);
    };

    if perp_dex_index == 0 {
        return Ok(index_in_meta);
    }

    let dex_offset = perp_dex_index
        .checked_mul(HYPERLIQUID_BUILDER_PERP_DEX_STRIDE)
        .ok_or_else(|| {
            InfraError::ApiCliError(format!(
                "Hyperliquid builder perp dex index overflow: {}",
                perp_dex_index
            ))
        })?;

    HYPERLIQUID_BUILDER_PERP_ASSET_OFFSET
        .checked_add(dex_offset)
        .and_then(|base| base.checked_add(index_in_meta))
        .ok_or_else(|| {
            InfraError::ApiCliError(format!(
                "Hyperliquid builder perp asset id overflow: dex_index={}, index_in_meta={}",
                perp_dex_index, index_in_meta
            ))
        })
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
    if let Some((base, quote)) = split_hyperliquid_cli_perp_inst(inst) {
        return hyperliquid_perp_parts_to_cli(base, quote);
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

    let (base, _) = split_hyperliquid_cli_perp_inst(&normalized_inst).ok_or_else(|| {
        InfraError::ApiCliError(format!(
            "Hyperliquid funding supports perpetual instruments only: {}",
            inst
        ))
    })?;

    Ok(hyperliquid_cli_symbol_to_raw_symbol(base))
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

pub fn hyperliquid_perp_to_cli(symbol: &str, quote: &str) -> String {
    hyperliquid_perp_parts_to_cli(hyperliquid_raw_perp_base(symbol), quote)
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

    if let Some((dex, base)) = coin.split_once(':') {
        let dex = dex.trim();
        let base = base.trim();

        if !dex.is_empty()
            && !base.is_empty()
            && let Some(quote) = hyperliquid_known_builder_perp_quote(dex)
        {
            return hyperliquid_perp_parts_to_cli(base, quote);
        }

        return coin.to_string();
    }

    hyperliquid_perp_to_cli(coin, HYPERLIQUID_QUOTE)
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
            if !is_hyperliquid_cli_perp_inst(&normalized) {
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

pub fn hyperliquid_cli_perp_quote(inst: &str) -> InfraResult<String> {
    let normalized_inst = normalize_hyperliquid_cli_inst(inst);
    let (_, quote) = split_hyperliquid_cli_perp_inst(&normalized_inst).ok_or_else(|| {
        InfraError::ApiCliError(format!(
            "Hyperliquid perpetual instrument must be BASE_QUOTE_PERP: {}",
            inst
        ))
    })?;

    Ok(hyperliquid_symbol_to_cli_symbol(quote))
}

pub fn is_hyperliquid_cli_perp_inst(inst: &str) -> bool {
    split_hyperliquid_cli_perp_inst(inst).is_some()
}

fn split_hyperliquid_cli_perp_inst(inst: &str) -> Option<(&str, &str)> {
    let base_quote = inst.strip_suffix(HYPERLIQUID_PERP_TYPE_SUFFIX)?;
    base_quote.rsplit_once('_')
}

fn hyperliquid_raw_perp_base(symbol: &str) -> &str {
    symbol
        .split_once(':')
        .map(|(_, base)| base)
        .unwrap_or(symbol)
}

fn hyperliquid_known_builder_perp_quote(dex: &str) -> Option<&'static str> {
    match dex.to_ascii_lowercase().as_str() {
        "xyz" | "abcd" | "para" => Some("USDC"),
        "flx" | "vntl" | "km" => Some("USDH"),
        "hyna" => Some("USDE"),
        "cash" => Some("USDT0"),
        _ => None,
    }
}

fn hyperliquid_perp_parts_to_cli(base: &str, quote: &str) -> String {
    format!(
        "{}_{}{}",
        hyperliquid_symbol_to_cli_symbol(base),
        hyperliquid_symbol_to_cli_symbol(quote),
        HYPERLIQUID_PERP_TYPE_SUFFIX
    )
}

#[derive(Clone, Debug, Serialize)]
pub struct HyperliquidOrderAction {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub orders: Vec<HyperliquidOrderRequest>,
    pub grouping: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder: Option<HyperliquidBuilderFee>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HyperliquidBuilderFee {
    pub b: String,
    pub f: u32,
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

pub fn hyperliquid_builder_fee_from_extra(
    extra: &HashMap<String, String>,
) -> InfraResult<Option<HyperliquidBuilderFee>> {
    let builder = extra.get(HYPERLIQUID_BUILDER_ADDRESS_EXTRA_KEY);
    let fee = extra.get(HYPERLIQUID_BUILDER_FEE_EXTRA_KEY);

    match (builder, fee) {
        (None, None) => Ok(None),
        (Some(builder), Some(fee)) => {
            let builder = normalize_hyperliquid_builder_address(builder)?;
            let f = fee.parse::<u32>().map_err(|_| {
                InfraError::ApiCliError(format!(
                    "Invalid Hyperliquid builder fee in OrderParams.extra[{}]: {}",
                    HYPERLIQUID_BUILDER_FEE_EXTRA_KEY, fee
                ))
            })?;
            Ok(Some(HyperliquidBuilderFee { b: builder, f }))
        },
        _ => Err(InfraError::ApiCliError(format!(
            "Hyperliquid builder fee requires both OrderParams.extra[{}] and OrderParams.extra[{}]",
            HYPERLIQUID_BUILDER_ADDRESS_EXTRA_KEY, HYPERLIQUID_BUILDER_FEE_EXTRA_KEY
        ))),
    }
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

fn normalize_hyperliquid_builder_address(address: &str) -> InfraResult<String> {
    let address = address.trim();
    let Some(hex) = address
        .strip_prefix("0x")
        .or_else(|| address.strip_prefix("0X"))
    else {
        return Err(InfraError::ApiCliError(format!(
            "Invalid Hyperliquid builder address, expected 0x-prefixed address: {}",
            address
        )));
    };

    if hex.len() != 40 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(InfraError::ApiCliError(format!(
            "Invalid Hyperliquid builder address: {}",
            address
        )));
    }

    Ok(format!("0x{}", hex.to_ascii_lowercase()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_perp_names_with_explicit_quote() {
        let cases = [
            ("BTC", "USDC", "BTC_USDC_PERP"),
            ("flx:OIL", "USDH", "OIL_USDH_PERP"),
            ("cash:WTI", "USDT0", "WTI_USDT0_PERP"),
        ];

        for (raw, quote, expected) in cases {
            assert_eq!(
                hyperliquid_perp_to_cli(raw, quote),
                expected,
                "raw={raw}, quote={quote}"
            );
        }
    }

    #[test]
    fn normalizes_hyperliquid_coin_ids_without_meta() {
        let cases = [
            ("BTC", "BTC_USDC_PERP"),
            ("kPEPE", "1000PEPE_USDC_PERP"),
            ("xyz:AAPL", "AAPL_USDC_PERP"),
            ("flx:OIL", "OIL_USDH_PERP"),
            ("vntl:OPENAI", "OPENAI_USDH_PERP"),
            ("hyna:BTC", "BTC_USDE_PERP"),
            ("km:GOLD", "GOLD_USDH_PERP"),
            ("abcd:TEST", "TEST_USDC_PERP"),
            ("cash:WTI", "WTI_USDT0_PERP"),
            ("para:AVGO", "AVGO_USDC_PERP"),
            ("newdex:ABC", "newdex:ABC"),
            ("@123", "@123"),
            ("PURR/USDC", "PURR_USDC"),
        ];

        for (raw, expected) in cases {
            assert_eq!(hyperliquid_inst_to_cli(raw), expected, "raw={raw}");
        }
    }

    #[test]
    fn parses_quote_from_base_quote_perp() {
        assert_eq!(hyperliquid_cli_perp_quote("OIL_USDH_PERP").unwrap(), "USDH");
    }

    #[test]
    fn computes_builder_dex_asset_id() {
        assert_eq!(
            hyperliquid_perp_asset_id_for_dex(7, Some(2)).unwrap(),
            120007
        );
    }
}
