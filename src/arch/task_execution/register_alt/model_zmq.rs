use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{sync::broadcast, time::timeout};
use zeromq::{ReqSocket, Socket, SocketRecv, SocketSend};

use crate::arch::{
    strategy_base::handler::{alt_events::AltTensor, handler_core::InfraMsg},
    task_execution::task_general::LogLevel,
};

use super::AltTaskBuilder;

impl AltTaskBuilder {
    pub(super) async fn model_preds_zmq(
        &mut self,
        tx: broadcast::Sender<InfraMsg<AltTensor>>,
        port: u64,
    ) {
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
            let Some(tensor) = self.recv_feat_input().await else {
                break;
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
                                self.emit_model_preds(&tx, matrix);
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
}
