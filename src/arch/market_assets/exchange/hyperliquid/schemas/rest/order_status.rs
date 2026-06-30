use serde::Deserialize;
use serde_json::Value;

use crate::arch::market_assets::{
    api_data::account_data::HistoOrderData,
    api_general::{ts_to_micros, value_to_f64},
    base_data::{OrderSide, OrderStatus, OrderType},
    exchange::hyperliquid::api_utils::{hyperliquid_inst_to_cli, hyperliquid_perp_to_cli},
};
use crate::errors::{InfraError, InfraResult};

const HYPERLIQUID_HISTORICAL_ORDERS_RECENT_LIMIT: usize = 2_000;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestOrderStatusHyperliquid {
    pub order: RestBasicOrderHyperliquid,
    pub status: String,
    pub statusTimestamp: u64,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestBasicOrderHyperliquid {
    pub coin: String,
    pub side: String,
    pub limitPx: Value,
    pub sz: Value,
    pub oid: u64,
    pub timestamp: u64,
    pub origSz: Value,
    #[serde(default)]
    pub cloid: Option<String>,
}

impl From<RestOrderStatusHyperliquid> for HistoOrderData {
    fn from(d: RestOrderStatusHyperliquid) -> Self {
        d.into_histo_order_data(None)
    }
}

impl RestOrderStatusHyperliquid {
    pub fn into_histo_order_data(self, perp_quote: Option<&str>) -> HistoOrderData {
        let d = self;
        let remaining_size = value_to_f64(&d.order.sz).abs();
        let orig_size = value_to_f64(&d.order.origSz).abs();
        let filled_size = (orig_size - remaining_size).max(0.0);
        let inst = match perp_quote {
            Some(quote) if !d.order.coin.contains('/') && !d.order.coin.starts_with('@') => {
                hyperliquid_perp_to_cli(&d.order.coin, quote)
            },
            _ => hyperliquid_inst_to_cli(&d.order.coin),
        };

        HistoOrderData {
            timestamp: ts_to_micros(d.order.timestamp),
            inst,
            order_id: d.order.oid.to_string(),
            cli_order_id: d.order.cloid.filter(|id| !id.is_empty()),
            side: match d.order.side.as_str() {
                "B" => OrderSide::BUY,
                "A" => OrderSide::SELL,
                _ => OrderSide::Unknown,
            },
            position_side: None,
            order_type: OrderType::Unknown,
            order_status: parse_order_status(&d.status, filled_size),
            price: value_to_f64(&d.order.limitPx),
            avg_price: 0.0,
            size: orig_size,
            executed_size: filled_size,
            fee: None,
            fee_currency: None,
            reduce_only: None,
            time_in_force: None,
            update_time: ts_to_micros(d.statusTimestamp.max(d.order.timestamp)),
        }
    }
}

fn parse_order_status(status: &str, filled_size: f64) -> OrderStatus {
    let status = status.to_ascii_lowercase();

    if status == "open" || status == "triggered" {
        if filled_size > 0.0 {
            OrderStatus::PartiallyFilled
        } else {
            OrderStatus::Live
        }
    } else if status == "filled" {
        OrderStatus::Filled
    } else if status.contains("cancel") {
        OrderStatus::Canceled
    } else if status.contains("reject") {
        OrderStatus::Rejected
    } else {
        OrderStatus::Unknown
    }
}

pub fn validate_hyperliquid_order_history_range(
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
) -> InfraResult<()> {
    if let Some(end_time_ms) = end_time_ms
        && let Some(start_time_ms) = start_time_ms
        && end_time_ms < start_time_ms
    {
        return Err(InfraError::ApiCliError(format!(
            "Hyperliquid order history end_time_ms {} is earlier than start_time_ms {}",
            end_time_ms, start_time_ms
        )));
    }

    Ok(())
}

pub fn finalize_hyperliquid_order_history(
    data: Vec<HistoOrderData>,
    normalized_inst: &str,
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
    limit: Option<u32>,
    require_recent_window_coverage: bool,
) -> InfraResult<Vec<HistoOrderData>> {
    if require_recent_window_coverage {
        ensure_recent_orders_cover_start(&data, start_time_ms)?;
    }

    Ok(filter_order_history(
        data,
        normalized_inst,
        start_time_ms,
        end_time_ms,
        limit,
    ))
}

fn order_history_time(order: &HistoOrderData) -> u64 {
    order.update_time.max(order.timestamp)
}

fn millis_to_micros(timestamp_ms: u64) -> u64 {
    timestamp_ms.saturating_mul(1_000)
}

fn filter_order_history(
    mut data: Vec<HistoOrderData>,
    normalized_inst: &str,
    start_time_ms: Option<u64>,
    end_time_ms: Option<u64>,
    limit: Option<u32>,
) -> Vec<HistoOrderData> {
    let start_time_us = start_time_ms.map(millis_to_micros);
    let end_time_us = end_time_ms.map(millis_to_micros);

    data.retain(|order| {
        if order.inst != normalized_inst {
            return false;
        }

        let order_time = order_history_time(order);
        start_time_us.is_none_or(|start| order_time >= start)
            && end_time_us.is_none_or(|end| order_time <= end)
    });

    data.sort_by_key(|order| std::cmp::Reverse(order_history_time(order)));
    if let Some(limit) = limit {
        data.truncate(limit as usize);
    }

    data
}

fn ensure_recent_orders_cover_start(
    data: &[HistoOrderData],
    start_time_ms: Option<u64>,
) -> InfraResult<()> {
    let Some(start_time_ms) = start_time_ms else {
        return Ok(());
    };

    if data.len() < HYPERLIQUID_HISTORICAL_ORDERS_RECENT_LIMIT {
        return Ok(());
    }

    let Some(earliest_returned_us) = data.iter().map(order_history_time).min() else {
        return Ok(());
    };

    let start_time_us = millis_to_micros(start_time_ms);
    if earliest_returned_us > start_time_us {
        return Err(InfraError::ApiCliError(format!(
            "Hyperliquid historicalOrders only returns the most recent {} orders; requested startTime={}ms is older than earliest returned order update_time={}ms",
            HYPERLIQUID_HISTORICAL_ORDERS_RECENT_LIMIT,
            start_time_ms,
            earliest_returned_us / 1_000
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn histo_order(inst: &str, order_id: &str, update_time: u64) -> HistoOrderData {
        HistoOrderData {
            timestamp: update_time.saturating_sub(1_000),
            inst: inst.to_string(),
            order_id: order_id.to_string(),
            cli_order_id: None,
            side: OrderSide::BUY,
            position_side: None,
            order_type: OrderType::Limit,
            order_status: OrderStatus::Filled,
            price: 1.0,
            avg_price: 1.0,
            size: 1.0,
            executed_size: 1.0,
            fee: None,
            fee_currency: None,
            reduce_only: None,
            time_in_force: None,
            update_time,
        }
    }

    #[test]
    fn filters_hyperliquid_order_history_by_inst_time_and_limit() {
        let data = vec![
            histo_order("BTC_USDC_PERP", "old", 1_000_000),
            histo_order("BTC_USDC_PERP", "middle", 2_000_000),
            histo_order("BTC_USDC_PERP", "new", 3_000_000),
            histo_order("ETH_USDC_PERP", "other", 3_500_000),
        ];

        let filtered = finalize_hyperliquid_order_history(
            data,
            "BTC_USDC_PERP",
            Some(1_500),
            Some(3_500),
            Some(2),
            true,
        )
        .unwrap();

        assert_eq!(
            filtered
                .iter()
                .map(|order| order.order_id.as_str())
                .collect::<Vec<_>>(),
            vec!["new", "middle"]
        );
    }

    #[test]
    fn rejects_time_range_older_than_full_recent_historical_orders_window() {
        let data = (0..HYPERLIQUID_HISTORICAL_ORDERS_RECENT_LIMIT)
            .map(|idx| histo_order("BTC_USDC_PERP", &idx.to_string(), 2_000_000 + idx as u64))
            .collect::<Vec<_>>();

        let err = finalize_hyperliquid_order_history(
            data,
            "BTC_USDC_PERP",
            Some(1_000),
            None,
            None,
            true,
        )
        .unwrap_err();

        assert!(format!("{err:?}").contains("historicalOrders only returns the most recent"));
    }
}
