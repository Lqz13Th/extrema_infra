#[derive(Clone, Debug, Default)]
pub enum Market {
    BinanceCmFutures,
    BinanceUmFutures,
    BinanceSpot,
    Coinbase,
    Okx,
    SolPumpFun,
    #[default]
    HyperLiquid,
}

