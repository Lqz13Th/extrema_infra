#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
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
