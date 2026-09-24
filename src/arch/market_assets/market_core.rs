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
    /// Venue implemented outside this crate.
    ///
    /// The id is the [`LobWsDecoder::ID`](crate::arch::traits::market_lob::LobWsDecoder::ID)
    /// of the decoder registered for its websocket tasks.
    Custom(u16),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_market_round_trips_through_serde() {
        let market = Market::Custom(7);
        let json = serde_json::to_string(&market).unwrap();

        assert_eq!(json, r#"{"Custom":7}"#);
        assert_eq!(serde_json::from_str::<Market>(&json).unwrap(), market);
    }

    #[test]
    fn custom_market_stays_small() {
        assert!(std::mem::size_of::<Market>() <= 4);
    }
}
