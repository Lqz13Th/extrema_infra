use std::sync::Arc;
use tokio::sync::oneshot;

use tracing::{error, info, warn};

use extrema_infra::prelude::*;
use extrema_infra::arch::market_assets::{
    exchange::prelude::OkxCli,
    api_general::OrderParams,
    base_data::{OrderSide, OrderType}
};

/// -------------------------------------
/// High Frequency Trading (HFT) Strategy
/// -------------------------------------
/// Responsibilities:
/// 1. Receive market data events (e.g., trades)
/// 2. Generate trading signals & features
/// 3. Receive predictions from model (message from python)
/// 4. Send order execution commands to AccountModule
#[derive(Clone)]
struct HFTStrategy {
    command_handles: Vec<Arc<CommandHandle>>,
}

impl HFTStrategy {
    pub fn new() -> Self {
        Self {
            command_handles: Vec::new(),
        }
    }

    /// Generate a signal from market trades.
    /// In this example, it creates a simple AltMatrix.
    async fn generate_signal(&mut self, msg: InfraMsg<Vec<WsTrade>>) -> InfraResult<()> {
        if msg.data.is_empty() {
            return Err(InfraError::Other("empty infra data".to_string()));
        }

        // Build feature matrix
        let n_rows = msg.data.len();
        let n_cols = 3;

        let feats: Vec<f32> = msg
            .data
            .iter()
            .map(|trade| (trade.price * trade.size) as f32)
            .collect();

        let matrix_a = AltMatrix {
            timestamp: 1234567890,
            data: feats,
            shape: vec![n_rows, n_cols],
        }.clone();

        let matrix_b = matrix_a.clone();

        // Send features to two different models
        self.send_feat_to_model_a(matrix_a).await?;
        self.send_feat_to_model_b(matrix_b).await?;
        Ok(())
    }

    /// Send feature matrix to model A
    async fn send_feat_to_model_a(&mut self, feat: AltMatrix) -> InfraResult<()> {
        if let Some(handle) = self.find_alt_handle(&AltTaskType::ModelPreds(1111), 1111) {
            let cmd = TaskCommand::FeatInput(feat);
            handle.send_command(cmd, None).await?;
        } else {
            error!("No model handle found for Model A");
        }
        Ok(())
    }

    /// Send feature matrix to model B
    async fn send_feat_to_model_b(&mut self, feat: AltMatrix) -> InfraResult<()> {
        if let Some(handle) = self.find_alt_handle(&AltTaskType::ModelPreds(2222), 2222) {
            let cmd = TaskCommand::FeatInput(feat);
            handle.send_command(cmd, None).await?;
        } else {
            error!("No model handle found for Model B");
        }
        Ok(())
    }

    /// Send order(s) to the order execution task
    /// Orders are sent via CommandHandle, not directly to the exchange
    async fn send_order(&mut self, orders: Vec<OrderParams>) -> InfraResult<()> {
        if let Some(handle) = self.find_alt_handle(&AltTaskType::OrderExecution, 1) {
            let cmd = TaskCommand::OrderExecute(orders);
            handle.send_command(cmd, None).await?;
        } else {
            error!("No order handle found");
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
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        self.command_handles.push(command_handle);
    }

    fn command_registry(&self) -> Vec<Arc<CommandHandle>> {
        self.command_handles.clone()
    }
}

impl EventHandler for HFTStrategy {
    /// Handle predictions from models
    async fn on_preds(&mut self, msg: InfraMsg<AltMatrix>) {
        info!("Received model prediction, task id: {}", msg.task_id);

        let order_params = OrderParams {
            inst: "BTC_USDT_PERP".to_string(),
            size: 0.01.to_string(),
            side: OrderSide::BUY,
            order_type: OrderType::Market,
            ..Default::default()
        };

        if let Err(e) = self.send_order(vec![order_params]).await {
            error!("Error sending order: {:?}", e);
        }
    }

    /// Subscribe and react to trades via WebSocket
    async fn on_trade(&mut self, msg: InfraMsg<Vec<WsTrade>>) {
        if let Err(e) = self.generate_signal(msg).await {
            error!("Error generating signal: {:?}", e);
        }
    }
}

/// -------------------------------------
/// Account Module
/// -------------------------------------
/// Responsibilities:
/// 1. Manage exchange connection and authentication
/// 2. Execute orders sent from strategies
/// 3. Receive account/order updates from exchange
#[derive(Clone)]
struct AccountModule {
    command_handles: Vec<Arc<CommandHandle>>,
    api_cli: OkxCli,
}

impl AccountModule {
    pub fn new() -> Self {
        Self {
            command_handles: Vec::new(),
            api_cli: OkxCli::default(),
        }
    }

    /// Connect and authenticate to OKX private WebSocket channels.
    /// This function performs multiple sequential async requests:
    /// - Connect
    /// - Login
    /// - Subscribe to account/order updates
    ///
    /// NOTE: This is an async/IO-heavy function, so it should be separated
    /// from latency-critical paths like signal processing.
    pub async fn connect_channel(&mut self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(channel, 1) {
            // Step 1: Connect
            let ws_url = self.api_cli.get_private_connect_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::Connect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle.send_command(cmd, Some((AckStatus::Connect, rx))).await?;

            // Step 2: Login
            warn!("okx api: {:?}", self.api_cli.api_key);
            let login_msg = self.api_cli.ws_login_msg()?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::Login {
                msg: login_msg,
                ack: AckHandle::new(tx),
            };
            handle.send_command(cmd, Some((AckStatus::Login, rx))).await?;

            // Step 3: Subscribe to private account/order updates
            let ws_msg = self.api_cli.get_private_sub_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::Subscribe {
                msg: ws_msg,
                ack: AckHandle::new(tx),
            };
            handle.send_command(cmd, Some((AckStatus::Subscribe, rx))).await?;

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
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        self.command_handles.push(command_handle);
    }

    fn command_registry(&self) -> Vec<Arc<CommandHandle>> {
        self.command_handles.clone()
    }
}

impl EventHandler for AccountModule {
    /// Handle incoming order execution requests from strategies.
    /// This places the order on OKX via REST/WebSocket API.
    async fn on_order_execution(&mut self, msg: InfraMsg<Vec<OrderParams>>) {
        for order in msg.data.iter() {
            self.api_cli
                .place_order(order.clone())
                .await
                .expect("order place failed");
        }
    }

    /// Handle private account WebSocket events (like order channel connect).
    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        if let Err(e) = self.connect_channel(&msg.data.ws_channel).await {
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
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    // Strategy logic (signal generation, sends orders to account module)
    let strategy_logic = HFTStrategy::new();
    // Account/order execution module (handles exchange connection + order placement)
    let strategy_account_module = AccountModule::new();

    // WebSocket tasks: account order updates & market trades
    let acc_order_task = WsTaskInfo {
        market: Market::Okx,
        ws_channel: WsChannel::AccountOrder,
        chunk: 1,
        task_id: None,
    };

    let okx_trade_task = WsTaskInfo {
        market: Market::Okx,
        ws_channel: WsChannel::Trades(Some(TradesParam::AggTrades)),
        chunk: 10, // Run 10 independent WebSocket connections for parallel trade feeds
        task_id: None,
    };

    let place_order_task = AltTaskInfo {
        alt_task_type: AltTaskType::OrderExecution,
        chunk: 1,
        task_id: None,
    };

    let model_a_task = AltTaskInfo {
        alt_task_type: AltTaskType::ModelPreds(1111), // Zeromq port
        chunk: 1,
        task_id: Some(1111), // Custom task ID
    };

    let model_b_task = AltTaskInfo {
        alt_task_type: AltTaskType::ModelPreds(2222), // Zeromq port
        chunk: 1,
        task_id: Some(2222), // Custom task ID
    };

    // EnvBuilder sets up the environment:
    // - Register broadcast channels (pub/sub for internal message passing)
    // - Register strategy modules
    // - Register WebSocket tasks
    let env = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_alt_event())
        .with_board_cast_channel(BoardCastChannel::default_ws_event())
        .with_board_cast_channel(BoardCastChannel::default_account_order())
        .with_board_cast_channel(BoardCastChannel::default_order_execution())
        .with_board_cast_channel(BoardCastChannel::default_model_preds())
        .with_strategy_module(strategy_account_module)
        .with_strategy_module(strategy_logic)
        .with_task(TaskInfo::WsTask(Arc::new(acc_order_task)))
        .with_task(TaskInfo::WsTask(Arc::new(okx_trade_task)))
        .with_task(TaskInfo::AltTask(Arc::new(model_a_task)))
        .with_task(TaskInfo::AltTask(Arc::new(model_b_task)))
        .with_task(TaskInfo::AltTask(Arc::new(place_order_task)))
        .build();

    // Execute environment (runs all tasks)
    env.execute().await;
}
