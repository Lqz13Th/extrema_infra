use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAccountConfigOkx {
    #[serde(default)]
    pub uid: Option<String>,
    #[serde(default)]
    pub mainUid: Option<String>,
    #[serde(default)]
    pub acctLv: Option<String>,
    #[serde(default)]
    pub posMode: Option<String>,
    #[serde(default)]
    pub autoLoan: Option<bool>,
    #[serde(default)]
    pub greeksType: Option<String>,
    #[serde(default)]
    pub feeType: Option<String>,
    #[serde(default)]
    pub acctStpMode: Option<String>,
    #[serde(default)]
    pub ctIsoMode: Option<String>,
    #[serde(default)]
    pub mgnIsoMode: Option<String>,
    #[serde(default)]
    pub spotOffsetType: Option<String>,
    #[serde(default)]
    pub roleType: Option<String>,
    #[serde(default)]
    pub spotRoleType: Option<String>,
    #[serde(default)]
    pub opAuth: Option<String>,
    #[serde(default)]
    pub kycLv: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub traderInsts: Option<Vec<String>>,
    #[serde(default)]
    pub enableSpotBorrow: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
