#[derive(Debug, Clone,PartialEq, Eq, Hash, Default)]
pub enum Market {
    #[default]
    BinanceUmFutures,
    BinanceSpot,
    Coinbase,
    OkxSwap,
    SolPumpFun,
}

