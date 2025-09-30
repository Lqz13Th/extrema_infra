use std::{
    sync::Arc,
    time::Duration,
};
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use extrema_infra::prelude::*;
use extrema_infra::market_assets::cex::prelude::*;

///---------------------------------------------------------
/// Empty Strategy
///---------------------------------------------------------
/// This is a placeholder strategy that:
/// - Initializes but does not trade
/// - Logs incoming events (candles, schedules)
///
/// Useful for testing system wiring without executing orders.
#[derive(Clone)]
struct EmptyStrategy;

impl EventHandler for EmptyStrategy {}
impl DexEventHandler for EmptyStrategy {}

impl Strategy for EmptyStrategy {
    async fn initialize(&mut self) {
        info!("[EmptyStrategy] Executing init strategy...");
    }

    fn strategy_name(&self) -> &'static str { "EmptyStrategy" }
}

impl AltEventHandler for EmptyStrategy {
    /// Called periodically if scheduled tasks are configured.
    async fn on_schedule(
        &mut self,
        msg: InfraMsg<AltScheduleEvent>,
    ) {
        info!("[EmptyStrategy] AltEventHandler: {:?}", msg);
    }
}

impl CexEventHandler for EmptyStrategy {
    /// Called when new candles are broadcasted.
    async fn on_candle(&mut self, msg: InfraMsg<Vec<WsCandle>>) {
        info!("[EmptyStrategy] Candle event: {:?}", msg);
    }
}

impl CommandEmitter for EmptyStrategy {
    /// Register command channel (not used in EmptyStrategy).
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategy] Command channel registered: {:?}", _command_handle);
    }

    /// No commands in this strategy.
    fn command_registry(&self) -> Vec<Arc<CommandHandle>> {
        Vec::new()
    }
}


///---------------------------------------------------------
/// Binance Strategy
///---------------------------------------------------------
/// This strategy demonstrates:
/// - Connecting to Binance UM Futures public WebSocket (candles, trades, etc.)
/// - Subscribing to BTC/USDT perpetual candles
/// - Receiving and logging events
///
/// Note: This strategy only listens/logs data, does not trade.
#[derive(Clone)]
struct BinanceStrategy {
    command_handles: Vec<Arc<CommandHandle>>,
    binance_um_cli: BinanceUmCli, // Public Binance UM Futures client (no API keys)
}

impl BinanceStrategy {
    fn new() -> Self {
        Self {
            command_handles: Vec::new(),
            binance_um_cli: BinanceUmCli::default(),
        }
    }

    /// Connect to Binance WebSocket channel and send subscription.
    /// This runs only when a CEX event is received that signals the channel is ready.
    async fn connect_channel(&self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(channel, 1) {
            info!("[BinanceStrategy] Sending connect to {:?}", handle);

            // Step 1: Request connection URL
            let ws_url = self.binance_um_cli.get_public_connect_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::Connect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle.send_command(cmd, Some((AckStatus::Connect, rx))).await?;

            // Step 2: Subscribe to BTC/USDT perpetual candle updates
            let ws_msg = self.binance_um_cli
                .get_public_sub_msg(channel, Some(&["BTC_USDT_PERP".into()]))
                .await?;

            let cmd = TaskCommand::Subscribe {
                msg: ws_msg,
                ack: AckHandle::none(), // no need to wait for ack
            };
            handle.send_command(cmd, None).await?;
        } else {
            warn!("[BinanceStrategy] No handle found for channel {:?}", channel);
        }

        Ok(())
    }
}

impl EventHandler for BinanceStrategy {}
impl DexEventHandler for BinanceStrategy {}

impl Strategy for BinanceStrategy {
    async fn initialize(&mut self) {
        info!("[BinanceStrategy] Starting strategy");
    }
}

impl AltEventHandler for BinanceStrategy {
    async fn on_schedule(
        &mut self,
        msg: InfraMsg<AltScheduleEvent>,
    ) {
        info!("[BinanceStrategy] AltEventHandler: {:?}", msg);
    }
}

impl CexEventHandler for BinanceStrategy {
    /// Triggered when a new WebSocket task is ready.
    /// Example: After creating WsTaskInfo for Binance Candle, this event will be fired
    /// and connect_channel() will be executed.
    async fn on_cex_event(&mut self, msg: InfraMsg<WsTaskInfo>)  {
        info!("[BinanceStrategy] Triggering connect for channel: {:?}", msg.data.ws_channel);
        if let Err(e) = self.connect_channel(&msg.data.ws_channel).await {
            error!("[BinanceStrategy] connect failed: {:?}", e);
        }
    }

    /// Handle incoming Binance candle data (1-minute candles here).
    async fn on_candle(&mut self, msg: InfraMsg<Vec<WsCandle>>) {
        info!("[BinanceStrategy] Candle event: {:?}", msg);
    }
}

impl CommandEmitter for BinanceStrategy {
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        info!("[BinanceStrategy] Command channel registered: {:?}", command_handle);
        self.command_handles.push(command_handle);
    }

    fn command_registry(&self) -> Vec<Arc<CommandHandle>> {
        self.command_handles.clone()
    }
}


///---------------------------------------------------------
/// Main Entry Point
///---------------------------------------------------------
/// - Initializes logging
/// - Creates strategies (EmptyStrategy + BinanceStrategy)
/// - Creates tasks (Binance candle WebSocket, Alt scheduler)
/// - Wires everything into EnvBuilder (pub/sub channels, strategies, tasks)
/// - Executes mediator event loop
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    // WebSocket Task: Binance Candle (1-minute)
    let binance_ws_candle = WsTaskInfo {
        market: Market::BinanceUmFutures,
        ws_channel: WsChannel::Candle(Some(CandleParam::OneMinute)),
        chunk: 1, // number of websocket connections for this task
        task_id: None,
    };

    // Alt Task: Time Scheduler (fires every 5 seconds)
    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(5)),
        chunk: 1,
        task_id: None,
    };

    // EnvBuilder builds the full runtime:
    // - Register broadcast channels (pub/sub message passing)
    // - Register strategy modules
    // - Register tasks
    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_cex_event())
        .with_board_cast_channel(BoardCastChannel::default_candle())
        .with_board_cast_channel(BoardCastChannel::default_candle()) // duplicated skip (can be removed)
        .with_board_cast_channel(BoardCastChannel::default_scheduler())
        .with_strategy_module(EmptyStrategy)
        .with_strategy_module(BinanceStrategy::new())
        .with_task(TaskInfo::WsTask(Arc::new(binance_ws_candle)))
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    // Start event loop (spawns all tasks, connects strategies, begins message flow)
    mediator.execute().await;
}
