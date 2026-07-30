#[cfg(feature = "model_onnx")]
mod model_onnx;

#[cfg(feature = "model_zmq")]
mod model_zmq;

use std::{sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{broadcast, mpsc},
    time::{interval, sleep},
};

use tracing::{error, info, warn};

#[cfg(any(feature = "model_onnx", feature = "model_zmq"))]
use super::task_alt::ModelRunner;
use super::{
    task_alt::{AltTaskInfo, AltTaskType},
    task_general::LogLevel,
};
use crate::arch::{
    market_assets::api_general::get_micros_timestamp,
    strategy_base::{
        command::{ack_handle::AckStatus, command_core::TaskCommand},
        handler::{
            alt_events::{AltScheduleEvent, AltTensor},
            task_channel::{InfraMsg, TaskEvent},
        },
    },
};

#[derive(Debug)]
pub(crate) struct AltTaskRunner {
    pub cmd_rx: mpsc::Receiver<TaskCommand>,
    pub event_tx: broadcast::Sender<TaskEvent>,
    pub alt_info: Arc<AltTaskInfo>,
    pub task_id: u64,
}

impl AltTaskRunner {
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    fn emit_model_preds(&self, tensor: AltTensor) {
        let _ = self.event_tx.send(TaskEvent::ModelPreds(InfraMsg {
            task_id: self.task_id,
            data: Arc::new(tensor),
        }));
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

    async fn order_execution(&mut self) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                TaskCommand::OrderExecute(alt_orders) => {
                    let _ = self.event_tx.send(TaskEvent::OrderExecute(InfraMsg {
                        task_id: self.task_id,
                        data: Arc::new(alt_orders),
                    }));
                },
                _ => self.handle_cmd(cmd),
            };
        }
    }

    async fn inst_intent(&mut self) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                TaskCommand::InstIntent(alt_intent) => {
                    let _ = self.event_tx.send(TaskEvent::InstIntent(InfraMsg {
                        task_id: self.task_id,
                        data: Arc::new(alt_intent),
                    }));
                },
                _ => self.handle_cmd(cmd),
            };
        }
    }

    async fn time_scheduler(&mut self, duration: Duration) {
        let mut interval = interval(duration);
        loop {
            select! {
                _ = interval.tick() => {
                    let _ = self.event_tx.send(
                        TaskEvent::Schedule(InfraMsg {
                            task_id: self.task_id,
                            data: Arc::new(AltScheduleEvent {
                                timestamp: get_micros_timestamp(),
                                duration,
                            }),
                        })
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
                self.order_execution().await;
            },
            AltTaskType::InstIntent => {
                self.inst_intent().await;
            },
            #[cfg(feature = "model_zmq")]
            AltTaskType::ModelPreds(ModelRunner::Zmq(port)) => {
                self.model_preds_zmq(port).await;
            },
            #[cfg(feature = "model_onnx")]
            AltTaskType::ModelPreds(ModelRunner::Onnx(config_path)) => {
                self.model_preds_onnx(config_path).await;
            },
            AltTaskType::TimeScheduler(duration) => {
                self.time_scheduler(duration).await;
            },
        };
    }

    fn alt_event(&self) {
        let msg = TaskEvent::Alt(InfraMsg {
            task_id: self.task_id,
            data: self.alt_info.clone(),
        });

        if let Err(e) = self.event_tx.send(msg) {
            self.log(LogLevel::Warn, &format!("Alt event send failed: {:?}", e));
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
