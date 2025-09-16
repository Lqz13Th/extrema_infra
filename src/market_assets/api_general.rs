use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};
use hmac::Hmac;
use sha2::Sha256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
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

pub fn get_timestamp() -> u64 {
    let now = SystemTime::now();
    let duration_since_epoch = now
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    duration_since_epoch.as_secs() * 1000 + duration_since_epoch.subsec_millis() as u64
}

pub fn build_query_string(args: HashMap<&str, &str>) -> String {
    form_urlencoded::Serializer::new(String::new())
        .extend_pairs(args)
        .finish()
}
