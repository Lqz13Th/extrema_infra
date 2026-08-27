use serde::Deserialize;
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestUserFeesHyperliquid {
    pub dailyUserVlm: Vec<RestDailyUserVolumeHyperliquid>,
    pub feeSchedule: RestFeeScheduleHyperliquid,
    pub userCrossRate: String,
    pub userAddRate: String,
    pub userSpotCrossRate: String,
    pub userSpotAddRate: String,
    pub activeReferralDiscount: String,
    #[serde(default)]
    pub trial: Option<Value>,
    pub feeTrialEscrow: String,
    #[serde(default)]
    pub nextTrialAvailableTimestamp: Option<u64>,
    #[serde(default)]
    pub stakingLink: Option<Value>,
    #[serde(default)]
    pub activeStakingDiscount: Option<RestStakingDiscountHyperliquid>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestDailyUserVolumeHyperliquid {
    pub date: String,
    pub userCross: String,
    pub userAdd: String,
    pub exchange: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestFeeScheduleHyperliquid {
    pub cross: String,
    pub add: String,
    pub spotCross: String,
    pub spotAdd: String,
    pub tiers: RestFeeTiersHyperliquid,
    pub referralDiscount: String,
    pub stakingDiscountTiers: Vec<RestStakingDiscountHyperliquid>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RestFeeTiersHyperliquid {
    pub vip: Vec<RestVipFeeTierHyperliquid>,
    pub mm: Vec<RestMarketMakerFeeTierHyperliquid>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestVipFeeTierHyperliquid {
    pub ntlCutoff: String,
    pub cross: String,
    pub add: String,
    pub spotCross: String,
    pub spotAdd: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestMarketMakerFeeTierHyperliquid {
    pub makerFractionCutoff: String,
    pub add: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct RestStakingDiscountHyperliquid {
    pub bpsOfMaxSupply: String,
    pub discount: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_fees() {
        let fees: RestUserFeesHyperliquid = serde_json::from_value(serde_json::json!({
            "dailyUserVlm": [{
                "date": "2026-08-27",
                "userCross": "12.34",
                "userAdd": "56.78",
                "exchange": "69.12"
            }],
            "feeSchedule": {
                "cross": "0.00045",
                "add": "0.00015",
                "spotCross": "0.0007",
                "spotAdd": "0.0004",
                "tiers": {
                    "vip": [{
                        "ntlCutoff": "5000000.0",
                        "cross": "0.0004",
                        "add": "0.00012",
                        "spotCross": "0.0006",
                        "spotAdd": "0.0003"
                    }],
                    "mm": [{
                        "makerFractionCutoff": "0.005",
                        "add": "-0.00001"
                    }]
                },
                "referralDiscount": "0.04",
                "stakingDiscountTiers": [{
                    "bpsOfMaxSupply": "0.0",
                    "discount": "0.05"
                }]
            },
            "userCrossRate": "0.0004275",
            "userAddRate": "0.0001425",
            "userSpotCrossRate": "0.000665",
            "userSpotAddRate": "0.00038",
            "activeReferralDiscount": "0.04",
            "trial": null,
            "feeTrialEscrow": "0.0",
            "nextTrialAvailableTimestamp": null,
            "stakingLink": null,
            "activeStakingDiscount": {
                "bpsOfMaxSupply": "0.0",
                "discount": "0.05"
            }
        }))
        .unwrap();

        assert_eq!(fees.dailyUserVlm.len(), 1);
        assert_eq!(fees.dailyUserVlm[0].date, "2026-08-27");
        assert_eq!(fees.dailyUserVlm[0].userCross, "12.34");
        assert_eq!(fees.dailyUserVlm[0].userAdd, "56.78");
        assert_eq!(fees.dailyUserVlm[0].exchange, "69.12");
        assert_eq!(fees.userCrossRate, "0.0004275");
        assert_eq!(fees.userAddRate, "0.0001425");
        assert_eq!(fees.userSpotCrossRate, "0.000665");
        assert_eq!(fees.userSpotAddRate, "0.00038");
        assert_eq!(fees.activeReferralDiscount, "0.04");
        assert!(fees.trial.is_none());
        assert_eq!(fees.feeTrialEscrow, "0.0");
        assert!(fees.nextTrialAvailableTimestamp.is_none());
        assert!(fees.stakingLink.is_none());

        assert_eq!(fees.feeSchedule.cross, "0.00045");
        assert_eq!(fees.feeSchedule.add, "0.00015");
        assert_eq!(fees.feeSchedule.spotCross, "0.0007");
        assert_eq!(fees.feeSchedule.spotAdd, "0.0004");
        assert_eq!(fees.feeSchedule.referralDiscount, "0.04");
        assert_eq!(fees.feeSchedule.tiers.vip.len(), 1);
        assert_eq!(fees.feeSchedule.tiers.vip[0].ntlCutoff, "5000000.0");
        assert_eq!(fees.feeSchedule.tiers.vip[0].cross, "0.0004");
        assert_eq!(fees.feeSchedule.tiers.vip[0].add, "0.00012");
        assert_eq!(fees.feeSchedule.tiers.vip[0].spotCross, "0.0006");
        assert_eq!(fees.feeSchedule.tiers.vip[0].spotAdd, "0.0003");
        assert_eq!(fees.feeSchedule.tiers.mm.len(), 1);
        assert_eq!(fees.feeSchedule.tiers.mm[0].makerFractionCutoff, "0.005");
        assert_eq!(fees.feeSchedule.tiers.mm[0].add, "-0.00001");
        assert_eq!(fees.feeSchedule.stakingDiscountTiers.len(), 1);
        assert_eq!(
            fees.feeSchedule.stakingDiscountTiers[0].bpsOfMaxSupply,
            "0.0"
        );
        assert_eq!(fees.feeSchedule.stakingDiscountTiers[0].discount, "0.05");

        let active_staking_discount = fees.activeStakingDiscount.unwrap();
        assert_eq!(active_staking_discount.bpsOfMaxSupply, "0.0");
        assert_eq!(active_staking_discount.discount, "0.05");
    }
}
