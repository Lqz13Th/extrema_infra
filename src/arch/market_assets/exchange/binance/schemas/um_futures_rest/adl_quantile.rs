use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RestPositionAdlQuantileBinanceUM {
    pub symbol: String,
    pub adlQuantile: RestAdlQuantileBinanceUM,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RestAdlQuantileBinanceUM {
    pub LONG: u8,
    pub SHORT: u8,
    #[serde(default)]
    pub BOTH: Option<u8>,
    #[serde(default)]
    pub HEDGE: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_one_way_and_cross_hedge_quantiles() {
        let one_way: RestPositionAdlQuantileBinanceUM = serde_json::from_str(
            r#"{"symbol":"BTCUSDT","adlQuantile":{"LONG":1,"SHORT":4,"BOTH":0}}"#,
        )
        .unwrap();
        assert_eq!(one_way.adlQuantile.BOTH, Some(0));
        assert_eq!(one_way.adlQuantile.HEDGE, None);

        let cross_hedge: RestPositionAdlQuantileBinanceUM = serde_json::from_str(
            r#"{"symbol":"ETHUSDT","adlQuantile":{"LONG":3,"SHORT":3,"HEDGE":2}}"#,
        )
        .unwrap();
        assert_eq!(cross_hedge.adlQuantile.BOTH, None);
        assert_eq!(cross_hedge.adlQuantile.HEDGE, Some(2));
    }
}
