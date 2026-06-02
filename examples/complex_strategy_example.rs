//! Larger runtime wiring example.
//!
//! This example combines public websocket data, model prediction tasks, order
//! execution relays, and an account module. It is intended to show how the
//! runtime pieces fit together rather than provide a production trading
//! strategy.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example complex_strategy_example --features okx
//! ```

use std::{collections::HashMap, sync::Arc};
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use extrema_infra::{
    arch::market_assets::{
        api_general::{OrderParams, get_micros_timestamp},
        base_data::{OrderSide, OrderType},
        exchange::prelude::OkxCli,
    },
    prelude::*,
};

/// Signal module for public trades and model predictions.
#[derive(Clone)]
struct HFTStrategy {
    command_registry: Arc<CommandRegistry>,
    api_cli: OkxCli,
}

impl HFTStrategy {
    pub fn new() -> Self {
        Self {
            command_registry: Arc::new(CommandRegistry::default()),
            api_cli: OkxCli::default(),
        }
    }

    /// Build an `AltTensor` feature batch from market trades.
    async fn generate_signal(&mut self, msg: InfraMsg<Vec<WsTrade>>) -> InfraResult<()> {
        if msg.data.is_empty() {
            return Err(InfraError::Msg("empty infra data".to_string()));
        }

        let n_rows = msg.data.len();
        let n_cols = 1;

        let feats: Vec<f32> = msg
            .data
            .iter()
            .map(|trade| (trade.price * trade.size) as f32)
            .collect();

        let matrix_a = AltTensor {
            timestamp: 1234567890,
            data: feats,
            shape: vec![n_rows, n_cols],
            metadata: Default::default(),
        }
        .clone();

        let matrix_b = matrix_a.clone();

        self.send_feat_to_model_a(matrix_a).await?;
        self.send_feat_to_model_b(matrix_b).await?;
        Ok(())
    }

    /// Send features to model A.
    async fn send_feat_to_model_a(&mut self, feat: AltTensor) -> InfraResult<()> {
        if let Some(handle) =
            self.find_alt_handle(&AltTaskType::ModelPreds(ModelRunner::Zmq(1111)), 1111)
        {
            let cmd = TaskCommand::FeatInput(feat);
            handle.send_command(cmd, None).await?;
        } else {
            error!("No model handle found for Model A");
        }
        Ok(())
    }

    /// Send features to model B.
    async fn send_feat_to_model_b(&mut self, feat: AltTensor) -> InfraResult<()> {
        if let Some(handle) =
            self.find_alt_handle(&AltTaskType::ModelPreds(ModelRunner::Zmq(2222)), 2222)
        {
            let cmd = TaskCommand::FeatInput(feat);
            handle.send_command(cmd, None).await?;
        } else {
            error!("No model handle found for Model B");
        }
        Ok(())
    }

    /// Forward orders to the order execution task.
    async fn send_order(&mut self, orders: Vec<AltOrder>) -> InfraResult<()> {
        if let Some(handle) = self.find_alt_handle(&AltTaskType::OrderExecution, 1) {
            let cmd = TaskCommand::OrderExecute(orders);
            handle.send_command(cmd, None).await?;
        } else {
            error!("No order handle found");
        }
        Ok(())
    }

    async fn connect_trade_channel(&self, channel: &WsChannel, task_id: u64) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(channel, task_id) {
            info!("Sending connect to {:?}", handle);

            let ws_url = self.api_cli.get_public_connect_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsConnect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsConnect, rx)))
                .await?;

            let ws_msg = self
                .api_cli
                .get_public_sub_msg(channel, Some(&["BTC_USDT_PERP".into()]))
                .await?;
            let cmd = TaskCommand::WsMessage {
                msg: ws_msg,
                ack: AckHandle::none(),
            };
            handle.send_command(cmd, None).await?;
        } else {
            warn!(
                "No handle found for channel {:?}, task_id={}",
                channel, task_id
            );
        }

        Ok(())
    }
}

impl Strategy for HFTStrategy {
    async fn initialize(&mut self) {
        info!("Initializing strategy");
    }
}
impl CommandEmitter for HFTStrategy {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.command_registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.command_registry.clone()
    }
}

impl EventHandler for HFTStrategy {
    fn event_mask(&self) -> EventMask {
        EventMask::WS_EVENT | EventMask::TRADE | EventMask::MODEL_PREDS
    }

    /// Handle predictions from models and generate orders.
    async fn on_preds(&mut self, msg: InfraMsg<AltTensor>) {
        info!("Received model prediction, task id: {}", msg.task_id);

        let order_params = OrderParams {
            inst: "BTC_USDT_PERP".to_string(),
            size: 0.01.to_string(),
            side: OrderSide::BUY,
            order_type: OrderType::Market,
            ..Default::default()
        };

        let mut order_info = HashMap::new();
        order_info.insert("acc_id".to_string(), "okx_test_01".to_string());

        let alt_order = AltOrder {
            timestamp: get_micros_timestamp(),
            market: Market::Okx,
            order_params,
            metadata: order_info,
        };

        if let Err(e) = self.send_order(vec![alt_order]).await {
            error!("Error sending order: {:?}", e);
        }
    }

    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        if msg.data.ws_channel == WsChannel::Trades(Some(TradesParam::AggTrades))
            && let Err(e) = self
                .connect_trade_channel(&msg.data.ws_channel, msg.task_id)
                .await
        {
            error!("connect ws public trade channel failed: {:?}", e);
        }
    }

    /// Handle public trade batches.
    async fn on_trade(&mut self, msg: InfraMsg<Vec<WsTrade>>) {
        if let Err(e) = self.generate_signal(msg).await {
            error!("Error generating signal: {:?}", e);
        }
    }
}

/// Account module for private websocket updates and execution requests.
#[derive(Clone)]
struct AccountModule {
    command_registry: Arc<CommandRegistry>,
    api_cli: OkxCli,
}

impl AccountModule {
    pub fn new() -> Self {
        Self {
            command_registry: Arc::new(CommandRegistry::default()),
            api_cli: OkxCli::default(),
        }
    }

    /// Connect, log in, and subscribe to the OKX private account channel.
    pub async fn connect_acc_channel(&mut self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(channel, 1) {
            let ws_url = self.api_cli.get_private_connect_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsConnect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsConnect, rx)))
                .await?;

            let login_msg = self.api_cli.ws_login_msg()?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsMessage {
                msg: login_msg,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsMessage, rx)))
                .await?;

            let ws_msg = self.api_cli.get_private_sub_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsMessage {
                msg: ws_msg,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsMessage, rx)))
                .await?;
        } else {
            warn!("No handle found for channel {:?}", channel);
        }
        Ok(())
    }
}

impl Strategy for AccountModule {
    async fn initialize(&mut self) {
        self.api_cli.init_api_key();
        info!("Starting order module, init okx api key");
    }
}

impl CommandEmitter for AccountModule {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        self.command_registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.command_registry.clone()
    }
}

impl EventHandler for AccountModule {
    fn event_mask(&self) -> EventMask {
        EventMask::WS_EVENT | EventMask::ORDER_EXECUTION | EventMask::ACC_ORDER
    }

    /// Receive order execution requests.
    async fn on_order_execution(&mut self, msg: InfraMsg<Vec<AltOrder>>) {
        info!("Received model order execution, task id: {}", msg.task_id);
        // Order submission is intentionally omitted in this example.
    }

    /// Handle private account WebSocket events.
    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        if msg.data.ws_channel == WsChannel::AccountOrders
            && let Err(e) = self.connect_acc_channel(&msg.data.ws_channel).await
        {
            error!("connect ws private account order channel failed: {:?}", e);
        }
    }

    /// Handle account/order updates from exchange
    async fn on_acc_order(&mut self, msg: InfraMsg<Vec<WsAccOrder>>) {
        info!("Updating account status: {:?}", msg);
    }
}

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    // Strategy logic (signal generation, sends orders to account module)
    let strategy_logic = HFTStrategy::new();
    // Account/order execution module (handles exchange connection + order placement)
    let strategy_account_module = AccountModule::new();

    // WebSocket tasks: account order updates & market trades
    let acc_order_task = WsTaskInfo {
        market: Market::Okx,
        ws_channel: WsChannel::AccountOrders,
        filter_channels: false,
        chunk: 1,
        task_base_id: None,
    };

    let okx_trade_task = WsTaskInfo {
        market: Market::Okx,
        ws_channel: WsChannel::Trades(Some(TradesParam::AggTrades)),
        filter_channels: false,
        chunk: 10, // Run 10 independent WebSocket connections for parallel trade feeds
        task_base_id: None,
    };

    let place_order_task = AltTaskInfo {
        alt_task_type: AltTaskType::OrderExecution,
        chunk: 1,
        task_base_id: None,
    };

    let model_a_task = AltTaskInfo {
        alt_task_type: AltTaskType::ModelPreds(ModelRunner::Zmq(1111)), // Zeromq port
        chunk: 1,
        task_base_id: Some(1111), // Custom task ID
    };

    let model_b_task = AltTaskInfo {
        alt_task_type: AltTaskType::ModelPreds(ModelRunner::Zmq(2222)), // Zeromq port
        chunk: 1,
        task_base_id: Some(2222), // Custom task ID
    };

    // EnvBuilder sets up the environment:
    // - Register broadcast channels (pub/sub for internal message passing)
    // - Register strategy modules
    // - Register WebSocket tasks
    let env = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_alt_event())
        .with_board_cast_channel(BoardCastChannel::default_ws_event())
        .with_board_cast_channel(BoardCastChannel::default_trade())
        .with_board_cast_channel(BoardCastChannel::default_account_order())
        .with_board_cast_channel(BoardCastChannel::default_order_execution())
        .with_board_cast_channel(BoardCastChannel::default_model_preds())
        .with_task(TaskInfo::WsTask(Arc::new(acc_order_task)))
        .with_task(TaskInfo::WsTask(Arc::new(okx_trade_task)))
        .with_task(TaskInfo::AltTask(Arc::new(model_a_task)))
        .with_task(TaskInfo::AltTask(Arc::new(model_b_task)))
        .with_task(TaskInfo::AltTask(Arc::new(place_order_task)))
        .with_strategy_module(strategy_account_module)
        .with_strategy_module(strategy_logic)
        .build();

    // Execute environment (runs all tasks)
    env.execute().await;
}
