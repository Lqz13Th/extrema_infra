use hmac::Hmac;
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;
use sha2::{Sha256, Sha512};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::arch::market_assets::base_data::{
    MarginMode, OrderSide, OrderType, PositionSide, TimeInForce,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature<T> {
    pub signature: String,
    pub timestamp: T,
}

pub type HmacSha256 = Hmac<Sha256>;
pub type HmacSha512 = Hmac<Sha512>;

pub enum RequestMethod {
    Get,
    Put,
    Post,
}

pub fn get_seconds_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
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
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        .unwrap_or(0.0)
}

pub fn normalize_to_string(value: f64, step: f64) -> String {
    if step <= 0.0 {
        return format!("{}", value);
    }

    let precision = step
        .to_string()
        .split('.')
        .nth(1)
        .map(|s| s.len())
        .unwrap_or(0);

    format!("{:.*}", precision, (value / step).floor() * step)
}

pub fn normalize_to_string_reduce_only(value: f64, step: f64) -> String {
    if step <= 0.0 {
        return format!("{}", value);
    }

    let precision = step
        .to_string()
        .split('.')
        .nth(1)
        .map(|s| s.len())
        .unwrap_or(0);

    format!("{:.*}", precision, (value / step).ceil() * step)
}

pub fn de_string_from_any<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(s) => Ok(s),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Null => Ok(String::new()),
        other => Err(de::Error::custom(format!(
            "invalid string type: {:?}",
            other
        ))),
    }
}

pub fn de_u64_from_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                Ok(u)
            } else if let Some(f) = n.as_f64() {
                Ok(f as u64)
            } else {
                Err(de::Error::custom("invalid u64 number"))
            }
        },
        Value::String(s) => s.trim().parse::<u64>().map_err(de::Error::custom),
        Value::Null => Ok(0),
        other => Err(de::Error::custom(format!("invalid u64 type: {:?}", other))),
    }
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
    pub time_in_force: Option<TimeInForce>, // GTC, IOC, FOK, GTD
    pub client_order_id: Option<String>,
    pub extra: HashMap<String, String>, // general
}
