use std::sync::Arc;
use std::time::Duration;
use rmp_serde::{Serializer, Deserializer};
use serde::{Serialize, Deserialize};
use tokio::{
    select,
    time::{
        sleep,
        interval,
    },
    sync::{
        mpsc,
        broadcast,
    }
};
use zeromq::{ReqSocket, Socket, SocketRecv, SocketSend};
use tracing::{error, info, warn};

use crate::market_assets::api_general::{
    get_micros_timestamp,
    OrderParams,
};
use crate::prelude::AltMatrix;
use crate::strategy_base::{
    command::{
        ack_handle::AckStatus,
        command_core::TaskCommand
    },
    handler::{
        alt_events::AltScheduleEvent,
        handler_core::*,
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
    pub task_numb: u64,
}


impl AltTaskBuilder {
    fn handle_cmd(&self, cmd: TaskCommand) {
        self.log(LogLevel::Warn, &format!("Unexpected command, auto-ack: {:?}", cmd));
        if let Some(ack_handle) = cmd.get_ack() {
            ack_handle.respond(AckStatus::AltTask);
        }
    }

    async fn order_execution(
        &mut self,
        tx: broadcast::Sender<InfraMsg<Vec<OrderParams>>>,
    ) {
        while let Some(cmd) = self.cmd_rx.recv().await {
            match cmd {
                TaskCommand::OrderExecute(order_params) => {
                    let _ = tx.send(
                        InfraMsg {
                            task_numb: self.task_numb,
                            data: Arc::new(order_params),
                        }
                    );
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
                            task_numb: self.task_numb,
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

    async fn model_preds(
        &mut self,
        tx: broadcast::Sender<InfraMsg<AltMatrix>>,
        port: u16,
    ) {
        let mut zmq_socket = ReqSocket::new();
        let address = format!("tcp://127.0.0.1:{}", port);
        if let Err(e) = zmq_socket.connect(&address).await {
            self.log(LogLevel::Error, &format!("ZMQ connect failed: {:?}", e));
            return;
        }

        loop {
            select! {
                recv_res = zmq_socket.recv() => match recv_res {
                    Ok(msg) => {
                        if let Some(bytes) = msg.get(0) {
                            let mut de = Deserializer::new(&bytes[..]);
                            match AltMatrix::deserialize(&mut de) {
                                Ok(matrix) => {
                                    let _ = tx.send(InfraMsg {
                                        task_numb: self.task_numb,
                                        data: Arc::new(matrix),
                                    });
                                },
                                Err(e) => {
                                    self.log(
                                        LogLevel::Error,
                                        &format!("Failed to deserialize ZMQ msg: {:?}", e)
                                    )
                                },
                            };
                        }
                    },
                    Err(e) => {
                        self.log(LogLevel::Error, &format!("ZMQ recv error: {:?}", e));
                        break;
                    },
                },
                cmd = self.cmd_rx.recv() => match cmd {
                    Some(TaskCommand::FeatInput(matrix)) => {
                        let mut buf = Vec::new();
                        if let Err(e) = matrix.serialize(&mut Serializer::new(&mut buf)) {
                            self.log(
                                LogLevel::Error,
                                &format!("Failed to serialize matrix: {:?}", e)
                            );
                            break;
                        }

                        if let Err(e) = zmq_socket.send(buf.into()).await {
                            self.log(
                                LogLevel::Error,
                                &format!("ZMQ send error: {:?}", e)
                            );
                            break;
                        }
                    },
                    Some(cmd) => self.handle_cmd(cmd),
                    None => {
                        self.log(LogLevel::Error, "Command channel closed");
                        break;
                    },
                }
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
        };
    }

    fn alt_event(&self) {
        if let Some(tx) = find_alt_event(&self.board_cast_channel) {
            let msg = InfraMsg {
                task_numb: self.task_numb,
                data: self.alt_info.clone(),
            };

            if let Err(e) = tx.send(msg) {
                self.log(LogLevel::Warn, &format!("Alt event send failed: {:?}", e));
            }
        } else {
            self.log(LogLevel::Warn, "No broadcast channel found for Alt event");
        }
    }

    pub(crate) async fn alt_mid_relay(
        &mut self,
    ) {
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