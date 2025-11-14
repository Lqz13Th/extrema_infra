use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct AltScheduleEvent {
    pub timestamp: u64,
    pub duration: Duration,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AltMatrix {
    pub timestamp: u64,    // Timestamp of the data (e.g., UNIX epoch)
    pub data: Vec<f32>,    // Flattened matrix stored as a 1D vector
    pub shape: Vec<usize>, // Shape of the matrix, length = number of dimensions (N-D)
}

