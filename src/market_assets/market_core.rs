#[derive(Clone, Debug, Default)]
pub enum Market {
    #[default]
    BinanceUmFutures,
    BinanceSpot,
    Coinbase,
    Okx,
    SolPumpFun,
}

