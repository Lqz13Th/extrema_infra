use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Market {
    #[default]
    HyperLiquid,
    BinanceCmFutures,
    BinanceSpot,
    BinanceUmFutures,
    Coinbase,
    GateDelivery,
    GateFutures,
    GateSpot,
    GateUni,
    Okx,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MarketScope {
    pub market: Market,
    pub extra: Option<String>,
}

impl MarketScope {
    pub fn new(market: Market, extra: Option<String>) -> Self {
        Self {
            market,
            extra: normalize_scope_extra(extra),
        }
    }

    pub fn default_for(market: Market) -> Self {
        Self {
            market,
            extra: None,
        }
    }
}

fn normalize_scope_extra(extra: Option<String>) -> Option<String> {
    extra
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
