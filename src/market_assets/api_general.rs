use std::time::{SystemTime, UNIX_EPOCH};
use hmac::Hmac;
use sha2::Sha256;
use serde::{Deserialize, Serialize};

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
        0..=999_999_999 => ts * 1_000_000,
        1_000_000_000..=999_999_999_999 => ts * 1_000,
        _ => ts,
    }
}
