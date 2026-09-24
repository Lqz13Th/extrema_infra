use std::borrow::Cow;

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
    /// Websocket tasks on a custom market are decoded by the
    /// [`LobWsDecoder`](crate::arch::traits::market_lob::LobWsDecoder)
    /// registered under the same name.
    Custom(Cow<'static, str>),
}

impl Market {
    /// Creates a custom market without allocating.
    pub const fn custom(name: &'static str) -> Self {
        Self::Custom(Cow::Borrowed(name))
    }
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
        let market = Market::custom("lighter");
        let json = serde_json::to_string(&market).unwrap();

        assert_eq!(json, r#"{"Custom":"lighter"}"#);
        assert_eq!(serde_json::from_str::<Market>(&json).unwrap(), market);
    }

    #[test]
    fn borrowed_and_owned_custom_names_are_equal() {
        assert_eq!(
            Market::custom("aster"),
            Market::Custom(Cow::Owned("aster".to_string()))
        );
    }
}
