use std::{
    fs::File,
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use hmac::Hmac;
use sha2::Sha256;
use serde::{
    de::DeserializeOwned,
    Deserialize, 
    Serialize,
};
use serde_json::Value;

use crate::errors::InfraResult;
use crate::arch::market_assets::base_data::{
    MarginMode, 
    OrderSide, 
    OrderType, 
    PositionSide, 
    TimeInForce,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature<T> {
    pub signature: String,
    pub timestamp: T,
}

pub type HmacSha256 = Hmac<Sha256>;

pub enum RequestMethod {
    Get,
    Put,
    Post,
}

pub fn read_keys_from_json<T: DeserializeOwned>(path: &str) -> InfraResult<HashMap<String, T>> {
    let file = File::open(path)?;
    let keys: HashMap<String, T> = serde_json::from_reader(file)?;
    Ok(keys)
}

pub fn get_mills_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

pub fn get_micros_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros() as u64
}

pub fn ts_to_micros(ts: u64) -> u64 {
    match ts {
        0..=9_999_999_999 => ts * 1_000_000,
        10_000_000_000..=9_999_999_999_999 => ts * 1_000,
        _ => ts,
    }
}

pub fn value_to_f64(v: &Value) -> f64 {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok())).unwrap_or(0.0)
}


#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OrderParams {
    pub inst: String,
    pub side: OrderSide,
    pub size: String,
    pub order_type: OrderType,
    pub price: Option<String>,
    pub reduce_only: Option<bool>,
    pub margin_mode: Option<MarginMode>,
    pub position_side: Option<PositionSide>,
    pub time_in_force: Option<TimeInForce>,  // GTC, IOC, FOK, GTD
    pub client_order_id: Option<String>,
    pub extra: HashMap<String, String>, // general
}
