pub use crate::arch::market_assets::exchange::{
    cex_clients::CexClients,
    binance::{
        binance_um_futures_cli::BinanceUmCli,
        binance_cm_futures_cli::BinanceCmCli,
        api_key::*,
        api_utils::*,
    },
    okx::{
        okx_cli::OkxCli,
        api_key::*,
        api_utils::*,
    },
};
