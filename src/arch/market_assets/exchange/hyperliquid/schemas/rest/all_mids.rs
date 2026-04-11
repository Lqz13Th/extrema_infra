use std::collections::HashMap;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct RestAllMidsHyperliquid(pub HashMap<String, String>);
