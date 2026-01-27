#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub enum Market {
    BinanceCmFutures,
    BinanceUmFutures,
    BinanceSpot,
    Coinbase,
    Gate,
    Okx,
    #[default]
    HyperLiquid,
}
