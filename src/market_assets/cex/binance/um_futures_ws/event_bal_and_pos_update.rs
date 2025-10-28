use serde::Deserialize;

use crate::market_assets::{
    api_general::ts_to_micros,
    cex::binance::api_utils::binance_inst_to_cli,
    base_data::*,
    market_core::Market,
};

use crate::strategy_base::handler::cex_events::{
    WsAccBalPos,
    WsAccBalance,
    WsAccPosition,
};
use crate::traits::conversion::IntoWsData;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WsBalAndPosBinanceUM {
    pub e: String,   // Event type, e.g. "ACCOUNT_UPDATE"
    pub E: u64,      // Event time (ms)
    pub T: u64,      // Transaction time (ms)
    pub a: AccountUpdate,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AccountUpdate {
    pub m: String,               // Event reason type (e.g. "ORDER", "FUNDING_FEE")
    pub B: Vec<AccountBalance>,  // Balances
    pub P: Vec<AccountPosition>, // Positions
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AccountBalance {
    pub a: String,  // Asset
    pub wb: String, // Wallet balance
    pub cw: String, // Cross wallet balance
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AccountPosition {
    pub s: String,  // Symbol
    pub pa: String, // Position amount
    pub ep: String, // Entry price
    pub cr: String, // (Cross) unrealized PnL
    pub up: String, // Unrealized profit
    pub mt: String, // Margin type
    pub iw: String, // Isolated wallet (if isolated)
    pub ps: String, // Position side ("BOTH", "LONG", "SHORT")
}

impl IntoWsData for WsBalAndPosBinanceUM {
    type Output = WsAccBalPos;
    fn into_ws(self) -> WsAccBalPos {
        let balances = self.a.B.into_iter().map(|b| WsAccBalance {
            inst: b.a,
            cash_bal: b.wb.parse::<f64>().unwrap_or_default(),
        }).collect();

        let positions = self.a.P.into_iter().map(|p| WsAccPosition {
            inst: binance_inst_to_cli(&p.s),
            inst_type: {
                if p.s.contains('_') {
                    InstrumentType::Futures
                } else {
                    InstrumentType::Perpetual
                }
            },
            size: p.pa.parse::<f64>().unwrap_or_default(),
            avg_price: p.ep.parse::<f64>().unwrap_or_default(),
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
        }).collect();

        WsAccBalPos {
            market: Market::BinanceUmFutures,
            timestamp: ts_to_micros(self.E),
            balances,
            positions,
        }
    }
}

