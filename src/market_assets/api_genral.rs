use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use crate::market_assets::base_data::Market;
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Signature<T> {
    pub signature: String,
    pub timestamp: T,
}

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
