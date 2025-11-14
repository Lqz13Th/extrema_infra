use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct AltScheduleEvent {
    pub timestamp: u64,
    pub duration: Duration,
}

#[derive(Clone, Debug)]
pub struct AltResample {
    pub timestamp: u64,
    pub inst: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AltMatrix {
    pub timestamp: u64,
    pub flat_matrix: Vec<f32>,
    pub n_rows: usize,
    pub n_cols: usize,
}
