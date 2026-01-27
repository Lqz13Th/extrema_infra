use serde::Deserialize;

use crate::arch::{
    market_assets::{
        api_general::ts_to_micros,
        base_data::{InstrumentType, MarginMode, PositionSide},
        exchange::binance::api_utils::binance_inst_to_cli,
        market_core::Market,
    },
    strategy_base::handler::lob_events::{WsAccBalPos, WsAccBalance, WsAccPosition},
    traits::conversion::IntoWsData,
};

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsBalAndPosBinanceUM {
    e: String, // Event type, e.g. "ACCOUNT_UPDATE"
    E: u64,    // Event time (ms)
    T: u64,    // Transaction time (ms)
    a: AccountUpdate,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct AccountUpdate {
    m: String,               // Event reason type (e.g. "ORDER", "FUNDING_FEE")
    B: Vec<AccountBalance>,  // Balances
    P: Vec<AccountPosition>, // Positions
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct AccountBalance {
    a: String,  // Asset
    wb: String, // Wallet balance
    cw: String, // Cross wallet balance
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct AccountPosition {
    s: String,  // Symbol
    pa: String, // Position amount
    ep: String, // Entry price
    cr: String, // (Cross) unrealized PnL
    up: String, // Unrealized profit
    mt: String, // Margin type
    iw: String, // Isolated wallet (if isolated)
    ps: String, // Position side ("BOTH", "LONG", "SHORT")
}

impl IntoWsData for WsBalAndPosBinanceUM {
    type Output = WsAccBalPos;
    fn into_ws(self) -> WsAccBalPos {
        let balances = self
            .a
            .B
            .into_iter()
            .map(|b| WsAccBalance {
                inst: b.a,
                balance: b.wb.parse().unwrap_or_default(),
            })
            .collect();

        let positions = self
            .a
            .P
            .into_iter()
            .map(|p| WsAccPosition {
                inst: binance_inst_to_cli(&p.s),
                inst_type: {
                    if p.s.contains('_') {
                        InstrumentType::Futures
                    } else {
                        InstrumentType::Perpetual
                    }
                },
                size: p.pa.parse().unwrap_or_default(),
                avg_price: p.ep.parse().unwrap_or_default(),
                position_side: match p.ps.as_str() {
                    "LONG" => PositionSide::Long,
                    "SHORT" => PositionSide::Short,
                    "BOTH" => PositionSide::Both,
                    _ => PositionSide::Unknown,
                },
                margin_mode: match p.mt.to_lowercase().as_str() {
                    "cross" => MarginMode::Cross,
                    "isolated" => MarginMode::Isolated,
                    _ => MarginMode::Unknown,
                },
            })
            .collect();

        WsAccBalPos {
            timestamp: ts_to_micros(self.E),
            market: Market::BinanceUmFutures,
            event: self.a.m,
            balances,
            positions,
        }
    }
}
