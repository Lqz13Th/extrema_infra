use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestUserRateLimitHyperliquid {
    pub cumVlm: String,
    pub nRequestsUsed: u64,
    pub nRequestsCap: u64,
    pub nRequestsSurplus: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_rate_limit() {
        let rate_limit: RestUserRateLimitHyperliquid = serde_json::from_value(serde_json::json!({
            "cumVlm": "1807.76",
            "nRequestsUsed": 1072,
            "nRequestsCap": 11807,
            "nRequestsSurplus": 0
        }))
        .unwrap();

        assert_eq!(rate_limit.cumVlm, "1807.76");
        assert_eq!(rate_limit.nRequestsUsed, 1072);
        assert_eq!(rate_limit.nRequestsCap, 11807);
        assert_eq!(rate_limit.nRequestsSurplus, 0);
    }
}
