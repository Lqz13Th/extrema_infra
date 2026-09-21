use serde::Deserialize;

/// Symbol-level ADL risk rating from `/fapi/v1/symbolAdlRisk`, recomputed every
/// 30 minutes. `adlRisk` is kept as a raw string: the endpoint returns
/// `LOW`, `MIDDLE`, `HIGH` and `EXTREMELY_HIGH`, while the docs still describe a
/// lowercase three-level `high`/`medium`/`low` scale, so callers must map it
/// themselves and treat unknown values as no signal. `updateTime` is the rating's
/// own computation time and can lag the current batch — one symbol staying behind
/// while the other ~768 advance is a state seen in production.
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RestSymbolAdlRiskBinanceUM {
    pub symbol: String,
    pub adlRisk: String,
    pub updateTime: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_full_list_and_the_single_symbol_object() {
        let list: Vec<RestSymbolAdlRiskBinanceUM> = serde_json::from_str(
            r#"[{"symbol":"BTCUSDT","adlRisk":"LOW","updateTime":1789961401613},
                {"symbol":"ACTUSDT","adlRisk":"EXTREMELY_HIGH","updateTime":1789959601613}]"#,
        )
        .unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].adlRisk, "LOW");
        assert_eq!(list[1].adlRisk, "EXTREMELY_HIGH");
        assert_eq!(list[1].updateTime, 1_789_959_601_613);

        let one: RestSymbolAdlRiskBinanceUM = serde_json::from_str(
            r#"{"symbol":"AKEUSDT","adlRisk":"HIGH","updateTime":1789961401613}"#,
        )
        .unwrap();
        assert_eq!(one.symbol, "AKEUSDT");
        assert_eq!(one.adlRisk, "HIGH");
    }

    #[test]
    fn keeps_undocumented_ratings_verbatim() {
        let one: RestSymbolAdlRiskBinanceUM =
            serde_json::from_str(r#"{"symbol":"BTCUSDT","adlRisk":"medium","updateTime":1}"#)
                .unwrap();
        assert_eq!(one.adlRisk, "medium");
    }
}
