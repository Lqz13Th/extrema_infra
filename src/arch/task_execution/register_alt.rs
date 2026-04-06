mod model_onnx;
mod model_zmq;

use std::{sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{broadcast, mpsc},
    time::{interval, sleep},
};

use tracing::{error, info, warn};

use super::{
    task_alt::{AltTaskInfo, AltTaskType, ModelRunner},
    task_general::LogLevel,
};
use crate::arch::{
    market_assets::api_general::get_micros_timestamp,
    strategy_base::{
        command::{ack_handle::AckStatus, command_core::TaskCommand},
        handler::{
            alt_events::{AltOrder, AltScheduleEvent, AltTensor},
            handler_core::*,
        },
    },
};
use crate::prelude::AltIntent;

#[derive(Debug)]
pub(crate) struct AltTaskBuilder {
    pub cmd_rx: mpsc::Receiver<TaskCommand>,
    pub board_cast_channel: Arc<Vec<BoardCastChannel>>,
    pub alt_info: Arc<AltTaskInfo>,
    pub task_id: u64,
}

impl AltTaskBuilder {
    async fn recv_feat_input(&mut self) -> Option<AltTensor> {
        loop {
            match self.cmd_rx.recv().await {
                Some(TaskCommand::FeatInput(tensor)) => return Some(tensor),
                Some(cmd) => self.handle_cmd(cmd),
                None => {
                    self.log(LogLevel::Error, "Command channel closed");
                    return None;
                },
            }
        }
    }

    fn emit_model_preds(&self, tx: &broadcast::Sender<InfraMsg<AltTensor>>, tensor: AltTensor) {
        let _ = tx.send(InfraMsg {
            task_id: self.task_id,
            data: Arc::new(tensor),
        });
    }

    fn handle_cmd(&self, cmd: TaskCommand) {
        self.log(
            LogLevel::Warn,
            &format!("Unexpected command, auto-ack: {:?}", cmd),
        );
        if let Some(ack_handle) = cmd.get_ack() {
            ack_handle.respond(AckStatus::AltTask);
        }
    }

    async fn order_execution(&mut self, tx: broadcast::Sender<InfraMsg<Vec<AltOrder>>>) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                TaskCommand::OrderExecute(alt_orders) => {
                    let _ = tx.send(InfraMsg {
                        task_id: self.task_id,
                        data: Arc::new(alt_orders),
                    });
                },
                _ => self.handle_cmd(cmd),
            };
        }
    }

    async fn inst_intent(&mut self, tx: broadcast::Sender<InfraMsg<AltIntent>>) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                TaskCommand::InstIntent(alt_intent) => {
                    let _ = tx.send(InfraMsg {
                        task_id: self.task_id,
                        data: Arc::new(alt_intent),
                    });
                },
                _ => self.handle_cmd(cmd),
            };
        }
    }

    async fn time_scheduler(
        &mut self,
        tx: broadcast::Sender<InfraMsg<AltScheduleEvent>>,
        duration: Duration,
    ) {
        let mut interval = interval(duration);
        loop {
            select! {
                _ = interval.tick() => {
                    let _ = tx.send(
                        InfraMsg {
                            task_id: self.task_id,
                            data: Arc::new(AltScheduleEvent {
                                timestamp: get_micros_timestamp(),
                                duration,
                            }),
                        }
                    );
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

    async fn alt_task_distribution(&mut self) {
        match self.alt_info.alt_task_type.clone() {
            AltTaskType::OrderExecution => {
                if let Some(tx) = find_order_execution(&self.board_cast_channel) {
                    self.order_execution(tx).await
                } else {
                    self.log(
                        LogLevel::Error,
                        "No broadcast channel found for order execution",
                    );
                }
            },
            AltTaskType::InstIntent => {
                if let Some(tx) = find_inst_intent(&self.board_cast_channel) {
                    self.inst_intent(tx).await
                } else {
                    self.log(
                        LogLevel::Error,
                        "No broadcast channel found for inst intent",
                    );
                }
            },
            AltTaskType::ModelPreds(ModelRunner::Zmq(port)) => {
                if let Some(tx) = find_model_preds(&self.board_cast_channel) {
                    self.model_preds_zmq(tx, port).await
                } else {
                    self.log(
                        LogLevel::Error,
                        "No broadcast channel found for model preds zmq",
                    );
                }
            },
            AltTaskType::ModelPreds(ModelRunner::Onnx(config_path)) => {
                if let Some(tx) = find_model_preds(&self.board_cast_channel) {
                    self.model_preds_onnx(tx, config_path).await
                } else {
                    self.log(
                        LogLevel::Error,
                        "No broadcast channel found for model preds onnx",
                    );
                }
            },
            AltTaskType::TimeScheduler(duration) => {
                if let Some(tx) = find_scheduler(&self.board_cast_channel) {
                    self.time_scheduler(tx, duration).await
                } else {
                    self.log(
                        LogLevel::Error,
                        "No broadcast channel found for time scheduler",
                    );
                }
            },
        };
    }

    fn alt_event(&self) {
        if let Some(tx) = find_alt_event(&self.board_cast_channel) {
            let msg = InfraMsg {
                task_id: self.task_id,
                data: self.alt_info.clone(),
            };

            if let Err(e) = tx.send(msg) {
                self.log(LogLevel::Warn, &format!("Alt event send failed: {:?}", e));
            }
        } else {
            self.log(LogLevel::Warn, "No broadcast channel found for Alt event");
        }
    }

    pub(crate) async fn alt_mid_relay(&mut self) {
        let sleep_interval = Duration::from_secs(5);
        self.log(LogLevel::Info, "Spawned alt task");
        loop {
            sleep(sleep_interval).await;
            self.alt_event();
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
