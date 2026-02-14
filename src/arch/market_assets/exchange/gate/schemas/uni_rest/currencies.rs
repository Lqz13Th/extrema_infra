use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct RestCurrenciesGateUnified {
    pub name: String,
    pub prec: String,
    pub min_borrow_amount: String,
    pub user_max_borrow_amount: String,
    pub total_max_borrow_amount: String,
    pub loan_status: String,
}
