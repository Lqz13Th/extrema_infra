use serde::Deserialize;


#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RestExchangeInfoBinanceCM {
    pub symbols: Vec<InstrumentInfo>,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct InstrumentInfo {
    pub symbol: String,
    pub contractType: String,
    pub status: String,
    pub pricePrecision: i32,
    pub quantityPrecision: i32,
    pub filters: Vec<Filter>
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) enum Filter {
    PRICE_FILTER(PriceFilter),
    // LOT_SIZE,
    // MARKET_LOT_SIZE,
    // MAX_NUM_ORDERS,
    // MAX_NUM_ALGO_ORDERS,
    // MIN_NOTIONAL,
    // PERCENT_PRICE,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PriceFilter {
    pub minPrice: String,
    pub tickSize: String,
    pub maxPrice: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LotSizeFilter {
    pub stepSize: String,
    pub tickSize: String,
    pub maxPrice: String,
}