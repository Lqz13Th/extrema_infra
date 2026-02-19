use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Market {
    #[default]
    HyperLiquid,
    BinanceCmFutures,
    BinanceUmFutures,
    BinanceSpot,
    Coinbase,
    GateDelivery,
    GateFutures,
    GateSpot,
    GateUni,
    Okx,
}
