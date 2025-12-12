use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{broadcast, mpsc},
    time::{interval, sleep, timeout},
};
use zeromq::{ReqSocket, Socket, SocketRecv, SocketSend};

use tracing::{error, info, warn};

use crate::arch::{
    market_assets::api_general::{OrderParams, get_micros_timestamp},
    strategy_base::{
        command::{ack_handle::AckStatus, command_core::TaskCommand},
        handler::{
            alt_events::{AltScheduleEvent, AltTensor},
            handler_core::*,
        },
    },
};

use super::{
    task_alt::{AltTaskInfo, AltTaskType},
    task_general::LogLevel,
};

#[derive(Debug)]
pub(crate) struct AltTaskBuilder {
    #[warn(dead_code)]
    pub cmd_rx: mpsc::Receiver<TaskCommand>,
    pub board_cast_channel: Arc<Vec<BoardCastChannel>>,
    pub alt_info: Arc<AltTaskInfo>,
    pub task_id: u64,
}

impl AltTaskBuilder {
    fn handle_cmd(&self, cmd: TaskCommand) {
        self.log(
            LogLevel::Warn,
            &format!("Unexpected command, auto-ack: {:?}", cmd),
        );
        if let Some(ack_handle) = cmd.get_ack() {
            ack_handle.respond(AckStatus::AltTask);
        }
    }

    async fn order_execution(&mut self, tx: broadcast::Sender<InfraMsg<Vec<OrderParams>>>) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                TaskCommand::OrderExecute(order_params) => {
                    let _ = tx.send(InfraMsg {
                        task_id: self.task_id,
                        data: Arc::new(order_params),
                    });
                },
                _ => self.handle_cmd(cmd),
            };
        }
    }

    async fn model_preds(&mut self, tx: broadcast::Sender<InfraMsg<AltTensor>>, port: u64) {
        let mut zmq_socket = ReqSocket::new();
        let address = format!("tcp://127.0.0.1:{}", port);

        self.log(
            LogLevel::Info,
            &format!("Connecting to model ZMQ server at {address}..."),
        );
        if let Err(e) = zmq_socket.connect(&address).await {
            self.log(LogLevel::Error, &format!("ZMQ connect failed: {:?}", e));
            return;
        }
        self.log(
            LogLevel::Info,
            &format!("Connected to model ZMQ server at {address}."),
        );

        let model_inference_timeout = Duration::from_secs(20);
        loop {
            let tensor = match self.cmd_rx.recv().await {
                Some(TaskCommand::FeatInput(t)) => t,
                Some(cmd) => {
                    self.handle_cmd(cmd);
                    continue;
                },
                None => {
                    self.log(LogLevel::Error, "Command channel closed");
                    break;
                },
            };

            let mut buf = Vec::new();
            if let Err(e) = tensor.serialize(&mut Serializer::new(&mut buf)) {
                self.log(
                    LogLevel::Error,
                    &format!("Failed to serialize tensor: {:?}", e),
                );
                break;
            }

            if let Err(e) = zmq_socket.send(buf.into()).await {
                self.log(LogLevel::Error, &format!("ZMQ send error: {:?}", e));
                break;
            }

            match timeout(model_inference_timeout, zmq_socket.recv()).await {
                Ok(Ok(msg)) => {
                    if let Some(bytes) = msg.get(0) {
                        let mut de = Deserializer::new(&bytes[..]);
                        match AltTensor::deserialize(&mut de) {
                            Ok(matrix) => {
                                let _ = tx.send(InfraMsg {
                                    task_id: self.task_id,
                                    data: Arc::new(matrix),
                                });
                            },
                            Err(e) => {
                                self.log(
                                    LogLevel::Error,
                                    &format!("Failed to deserialize ZMQ msg: {:?}", e),
                                );
                            },
                        };
                    } else {
                        self.log(LogLevel::Error, "ZMQ msg had no frame");
                    }
                },
                Ok(Err(e)) => {
                    self.log(LogLevel::Error, &format!("ZMQ recv error: {:?}", e));
                    break;
                },
                Err(_) => {
                    self.log(
                        LogLevel::Warn,
                        "Model prediction TIMEOUT - skipping this tick",
                    );
                    continue;
                },
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
        match self.alt_info.alt_task_type {
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
            AltTaskType::ModelPreds(port) => {
                if let Some(tx) = find_model_preds(&self.board_cast_channel) {
                    self.model_preds(tx, port).await
                } else {
                    self.log(
                        LogLevel::Error,
                        "No broadcast channel found for model preds",
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
