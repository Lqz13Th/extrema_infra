use serde::Deserialize;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Default, Deserialize)]
pub struct RestAssetCurrencyOkx {
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub logoLink: String,
    #[serde(default)]
    pub chain: String,
    #[serde(default)]
    pub ctAddr: String,
    #[serde(default)]
    pub canDep: bool,
    #[serde(default)]
    pub canWd: bool,
    #[serde(default)]
    pub canInternal: bool,
    #[serde(default)]
    pub depQuotaFixed: String,
    #[serde(default)]
    pub usedDepQuotaFixed: String,
    #[serde(default)]
    pub depQuoteDailyLayer2: String,
    #[serde(default)]
    pub wdQuota: String,
    #[serde(default)]
    pub usedWdQuota: String,
    #[serde(default)]
    pub minDep: String,
    #[serde(default)]
    pub minWd: String,
    #[serde(default)]
    pub minInternal: String,
    #[serde(default)]
    pub minFee: String,
    #[serde(default)]
    pub maxFee: String,
    #[serde(default)]
    pub minFeeForCtAddr: String,
    #[serde(default)]
    pub maxFeeForCtAddr: String,
    #[serde(default)]
    pub burningFeeRate: String,
    #[serde(default)]
    pub fee: String,
    #[serde(default)]
    pub wdTickSz: String,
    #[serde(default)]
    pub wdQuotaIncrement: String,
    #[serde(default)]
    pub mainNet: bool,
    #[serde(default)]
    pub needTag: bool,
    #[serde(default)]
    pub minDepArrivalConfirm: String,
    #[serde(default)]
    pub minWdUnlockConfirm: String,
    #[serde(default)]
    pub depWthAwlSlogan: String,
}
