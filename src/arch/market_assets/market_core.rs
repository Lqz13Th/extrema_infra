#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub enum Market {
    #[default]
    HyperLiquid,
    BinanceCmFutures,
    BinanceUmFutures,
    BinanceSpot,
    Coinbase,
    Gate,
    Okx,
}
