use std::sync::Arc;
use std::time::Duration;

use tokio::{
    time::sleep,
    sync::mpsc
};
use tracing::{error, info, warn};

use crate::market_assets::api_general::get_micros_timestamp;
use crate::strategy_base::{
    command::command_core::TaskCommand,
    handler::{
        alt_events::AltTimerEvent,
        handler_core::{find_timer, BoardCastChannel}
    }
};
use super::{
    task_general::LogLevel,
    task_alt::{AltTaskInfo, AltTaskType}
};




#[derive(Debug)]
pub(crate) struct AltTaskBuilder {
    #[warn(dead_code)]
    pub cmd_rx: mpsc::Receiver<TaskCommand>,
    pub board_cast_channel: Arc<Vec<BoardCastChannel>>,
    pub alt_info: Arc<AltTaskInfo>,
}


impl AltTaskBuilder {
    fn handle_cmd(&self, cmd: TaskCommand) {
        self.log(LogLevel::Warn, &format!("Unexpected command, auto-ack: {:?}", cmd));
        if let Some(ack) = cmd.ack() {
            ack.respond(Ok(()));
        }
    }

    async fn timer_based_state(&mut self, n: u64) {
        let mut interval = tokio::time::interval(Duration::from_secs(n));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Some(tx) = find_timer(&self.board_cast_channel) {
                        let timer_event = AltTimerEvent {
                            timestamp: get_micros_timestamp(),
                            interval_sec: n,
                        };
                        let _ = tx.send(Arc::new(timer_event));
                    } else {
                        self.log(LogLevel::Warn, "No timer channel found, retrying...");
                    }
                },
                result = self.cmd_rx.recv() => {
                    match result {
                        Some(cmd) => self.handle_cmd(cmd),
                        None => {
                            self.log(LogLevel::Error, "Command channel closed");
                            break;
                        },
                    };
                },
            }
        }
    }


    fn neural_network(&self) {
        println!("neural network");
    }

    async fn alt_task_distribution(&mut self) {
        match self.alt_info.alt_task_type {
            AltTaskType::NeuralNetwork(n) => {
                self.neural_network();
                self.log(LogLevel::Warn, &format!("Unimplemented NeuralNetwork({})", n));
            },
            AltTaskType::TimerBasedState(n) => {
            self.timer_based_state(n).await;
            },
        };
    }

    pub(crate) async fn alt_mid_relay(
        &mut self,
    ) {
        let sleep_interval = Duration::from_secs(5);
        self.log(LogLevel::Info, "Spawned alt task");
        loop {
            sleep(sleep_interval).await;
            self.log(LogLevel::Info, "Initiated");
            self.alt_task_distribution().await;
        }
    }

    fn log(&self, level: LogLevel, msg: &str) {
        match level {
            LogLevel::Info => {
                info!("Alt task: {:?}. {}", self.alt_info, msg)
            },
            LogLevel::Warn => {
                warn!("Alt task: {:?}. {}", self.alt_info, msg)
            },
            LogLevel::Error => {
                error!("Alt task: {:?}. {}", self.alt_info, msg)
            },
        }
    }
}