use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct AltScheduleEvent {
    pub timestamp: u64,
    pub duration: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AltMatrix {
    pub timestamp: u64,
    pub preds: Vec<f32>,
    pub n_rows: usize,
    pub n_cols: usize,
}
