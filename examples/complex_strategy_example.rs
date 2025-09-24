use std::sync::Arc;
use tokio::sync::oneshot;

use tracing::{error, info, warn};

use extrema_infra::prelude::*;
use extrema_infra::market_assets::{
    cex::prelude::OkxCli,
    api_general::OrderParams,
    base_data::{OrderSide, OrderType}
};

/// High Frequency Trading (HFT) Strategy
/// -------------------------------------
/// This module is responsible for:
/// 1. Receiving market data events (e.g., trades)
/// 2. Generating trading signals
/// 3. Sending order execution commands to the Order/Account module
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

    /// Generate a simple signal (here: always buy 0.01 BTC at market price).
    /// In practice, you would add your cpu bounded logic here.
    async fn generate_signal(&mut self) -> InfraResult<()> {
        let order_params = OrderParams {
            inst: "BTC_USDT_PERP".to_string(),
            size: 0.01.to_string(),
            side: OrderSide::BUY,
            order_type: OrderType::Market,
            ..Default::default()
        };

        self.send_order(vec![order_params]).await
    }

    /// Send order(s) to the order execution task.
    /// This does not place orders directly; instead it sends a command to the
    /// AccountModule, which is responsible for communicating with the exchange.
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

impl EventHandler for HFTStrategy {}
impl AltEventHandler for HFTStrategy {}
impl DexEventHandler for HFTStrategy {}

impl Strategy for HFTStrategy {
    async fn initialize(&mut self) {
        info!("Initializing strategy");
    }
}

impl CexEventHandler for HFTStrategy {
    /// You should send subscribe message to websocket in order to get trade msg.
    /// React to new trades (market data).
    /// Each trade event may trigger signal generation.
    async fn on_trade(&mut self, _msg: InfraMsg<Vec<WsTrade>>) {
        if let Err(e) = self.generate_signal().await {
            error!("Error generating signal: {:?}", e);
        }
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


/// Account Module
/// --------------
/// This module is responsible for:
/// 1. Managing exchange connection (login, subscriptions, heartbeats).
/// 2. Executing incoming orders (from strategy).
/// 3. Receiving account/order updates from the exchange.
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
    /// ⚠️ NOTE: This is an async/IO-heavy function, so it should be separated
    /// from latency-critical paths like signal processing.
    pub async fn connect_channel(&mut self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(&channel, 1) {
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

impl EventHandler for AccountModule {}
impl DexEventHandler for AccountModule {}

impl Strategy for AccountModule {
    async fn initialize(&mut self) {
        self.api_cli.init_api_key();
        info!("Starting order module, init okx api key");
    }
}

impl AltEventHandler for AccountModule {
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
}

impl CexEventHandler for AccountModule {
    /// Handle private account WebSocket events (like order channel connect).
    async fn on_cex_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        if let Err(e) = self.connect_channel(&msg.data.ws_channel).await {
            error!("connect ws private account order channel failed: {:?}", e);
        }
    }

    /// Handle account/order updates from OKX (fills, cancellations, etc.).
    async fn on_acc_order(&mut self, msg: InfraMsg<Vec<WsAccOrder>>) {
        info!("Updating account status: {:?}", msg);
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
        chunk: 1
    };

    let okx_trade_task = WsTaskInfo {
        market: Market::Okx,
        ws_channel: WsChannel::Trades(Some(TradesParam::AggTrades)),
        chunk: 10 // Run 10 independent WebSocket connections for parallel trade feeds
    };

    // EnvBuilder sets up the environment:
    // - Register broadcast channels (pub/sub for internal message passing)
    // - Register strategy modules
    // - Register WebSocket tasks
    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_cex_event())
        .with_board_cast_channel(BoardCastChannel::default_account_order())
        .with_board_cast_channel(BoardCastChannel::default_order_execution())
        .with_board_cast_channel(BoardCastChannel::default_cex_event())
        .with_strategy_module(strategy_account_module)
        .with_strategy_module(strategy_logic)
        .with_task(TaskInfo::WsTask(Arc::new(acc_order_task)))
        .with_task(TaskInfo::WsTask(Arc::new(okx_trade_task)))
        .build();

    // Run the environment (spins up event loop + tasks)
    mediator.execute().await;
}
