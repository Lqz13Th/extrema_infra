use std::time::Duration;

#[derive(Clone, Debug)]
pub struct AltScheduleEvent {
    pub timestamp: u64,
    pub duration: Duration,
}