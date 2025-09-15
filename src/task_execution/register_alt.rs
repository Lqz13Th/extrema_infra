use std::sync::Arc;
use std::time::Duration;

use tokio::{
    time::sleep,
    sync::{broadcast, mpsc}
};

use tracing::{error, info, warn};

use crate::errors::{InfraError, InfraResult};
use crate::strategy_base::{
    command::command_core::TaskCommand,
    handler::handler_core::{find_timer, BoardCastChannel}
};
use super::{
    task_general::LogLevel,
    task_alt::{AltTaskInfo, AltTaskType}
};




#[derive(Debug)]
pub(crate) struct AltTaskBuilder {
    #[warn(dead_code)]
    pub(crate) cmd_rx: mpsc::Receiver<TaskCommand>,
    pub(crate) board_cast_channel: Arc<Vec<BoardCastChannel>>,
    pub(crate) alt_info: Arc<AltTaskInfo>,
}


impl AltTaskBuilder {
    async fn consume_command(&mut self) {
        if let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                TaskCommand::Connect { msg, ack } => {
                    self.log(LogLevel::Info, &format!("Received Connect: {}", msg));
                    ack.respond(Ok(()));
                },
                TaskCommand::Subscribe { msg, ack } |
                TaskCommand::Unsubscribe { msg, ack } |
                TaskCommand::Shutdown { msg, ack } => {
                    self.log(LogLevel::Warn, &format!("Unexpected command: {:?}", msg));
                    ack.respond(Ok(()));

                },
                _ => {
                    self.log(LogLevel::Warn, &format!("Unexpected command: {:?}", cmd));
                }
            }
        } 
    }

    fn timer_based_state(&self) {
        println!("timer");
        if let Some(tx) = find_timer(&self.board_cast_channel) {
            let _ = tx.send(());
        } else {
            self.log(LogLevel::Warn, "No timer channel found, retrying...");
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
                loop {
                    self.timer_based_state();
                    self.consume_command().await;
                    sleep(Duration::from_millis(n)).await;
                }
            },
        };
    }

    pub(crate) async fn alt_mid_relay(
        &mut self,
    ) {
        let sleep_interval = Duration::from_secs(1);
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