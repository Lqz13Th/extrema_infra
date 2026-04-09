use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestCapitalConfigNetworkBinance {
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub coin: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub depositEnable: bool,
    #[serde(default)]
    pub withdrawEnable: bool,
    #[serde(default)]
    pub isDefault: bool,
    #[serde(default)]
    pub withdrawFee: String,
    #[serde(default)]
    pub withdrawMin: String,
    #[serde(default)]
    pub withdrawMax: String,
    #[serde(default)]
    pub withdrawIntegerMultiple: String,
    #[serde(default)]
    pub withdrawInternalMin: String,
    #[serde(default)]
    pub depositDust: String,
    #[serde(default)]
    pub busy: bool,
    #[serde(default)]
    pub addressRegex: String,
    #[serde(default)]
    pub memoRegex: String,
    #[serde(default)]
    pub withdrawTag: bool,
    #[serde(default)]
    pub resetAddressStatus: bool,
    #[serde(default)]
    pub sameAddress: bool,
    #[serde(default)]
    pub depositDesc: String,
    #[serde(default)]
    pub withdrawDesc: String,
    #[serde(default)]
    pub specialTips: String,
    #[serde(default)]
    pub specialWithdrawTips: String,
    #[serde(default)]
    pub contractAddress: String,
    #[serde(default)]
    pub contractAddressUrl: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestCapitalConfigCoinBinance {
    #[serde(default)]
    pub coin: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub depositAllEnable: bool,
    #[serde(default)]
    pub withdrawAllEnable: bool,
    #[serde(default)]
    pub free: String,
    #[serde(default)]
    pub locked: String,
    #[serde(default)]
    pub freeze: String,
    #[serde(default)]
    pub withdrawing: String,
    #[serde(default)]
    pub ipoing: String,
    #[serde(default)]
    pub ipoable: String,
    #[serde(default)]
    pub storage: String,
    #[serde(default)]
    pub isLegalMoney: bool,
    #[serde(default)]
    pub trading: bool,
    #[serde(default)]
    pub networkList: Vec<RestCapitalConfigNetworkBinance>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
