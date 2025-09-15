use std::sync::Arc;
use std::time::Duration;

use tokio::{
    time::sleep,
    sync::{broadcast, mpsc}
};
use tokio::sync::mpsc::error::TryRecvError;
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
        match self.cmd_rx.try_recv() {
            Ok(cmd) => {
                println!("111111{:?}", cmd);
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
                        self.log(LogLevel::Warn, &format!("abs Unexpected command: {:?}", cmd));
                    }
                }
            },
            Err(TryRecvError::Empty) => {
            },
            Err(TryRecvError::Disconnected) => {
                self.log(LogLevel::Error, "Command channel disconnected");
            },
        };
    }

    async fn timer_based_state(&mut self, n: u64) {
        loop {
            tokio::select! {
                _ = sleep(Duration::from_millis(n)) => {
                    println!("timer");
                    if let Some(tx) = find_timer(&self.board_cast_channel) {
                        let _ = tx.send(());
                    } else {
                        self.log(LogLevel::Warn, "No timer channel found, retrying...");
                    }
                },
                result = self.cmd_rx.recv() => {
                    match result {
                        Some(cmd) => {
                            self.log(
                                    LogLevel::Warn,
                                    &format!("Unexpected command, auto-ack: {:?}", cmd)
                                );
                            if let Some(ack) = cmd.ack() {
                                ack.respond(Ok(()));
                            }
                        },
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