//! Multiple strategy modules in one runtime.
//!
//! Runs a scheduler module and a Binance public candle module in one runtime.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example multi_strategy_example --features binance
//! ```

use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use extrema_infra::{arch::market_assets::exchange::prelude::*, prelude::*};

/// Scheduler and candle logger.
#[derive(Clone)]
struct EmptyStrategy;

impl Strategy for EmptyStrategy {
    async fn initialize(&mut self) {
        info!("[EmptyStrategy] Executing init strategy...");
    }

    fn strategy_name(&self) -> &'static str {
        "EmptyStrategy"
    }
}

impl CommandEmitter for EmptyStrategy {
    /// Command registry is not used by this logger.
    fn command_init(&mut self, _registry: Arc<CommandRegistry>) {
        info!(
            "[EmptyStrategy] Command channel registered: {:?}",
            _registry
        );
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        Arc::new(CommandRegistry::default())
    }
}

impl EventHandler for EmptyStrategy {
    async fn on_schedule(&mut self, msg: InfraMsg<AltScheduleEvent>) {
        info!("[EmptyStrategy] AltEventHandler: {:?}", msg);
    }

    async fn on_candle(&mut self, msg: InfraMsg<Vec<WsCandle>>) {
        info!("[EmptyStrategy] Candle event: {:?}", msg);
    }
}

/// Binance public candle subscriber.
#[derive(Clone)]
struct BinanceStrategy {
    command_registry: Arc<CommandRegistry>,
    binance_um_cli: BinanceUmCli, // Public Binance UM Futures client (no API keys)
}

impl BinanceStrategy {
    fn new() -> Self {
        Self {
            command_registry: Arc::new(CommandRegistry::default()),
            binance_um_cli: BinanceUmCli::default(),
        }
    }

    /// Connect after the websocket task emits `on_ws_event`.
    async fn connect_channel(&self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(channel, 1) {
            info!("[BinanceStrategy] Sending connect to {:?}", handle);

            let ws_url = self.binance_um_cli.get_public_connect_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsConnect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsConnect, rx)))
                .await?;

            let ws_msg = self
                .binance_um_cli
                .get_public_sub_msg(channel, Some(&["BTC_USDT_PERP".into()]))
                .await?;

            let cmd = TaskCommand::WsMessage {
                msg: ws_msg,
                ack: AckHandle::none(),
            };
            handle.send_command(cmd, None).await?;
        } else {
            warn!(
                "[BinanceStrategy] No handle found for channel {:?}",
                channel
            );
        }

        Ok(())
    }
}

impl Strategy for BinanceStrategy {
    async fn initialize(&mut self) {
        info!("[BinanceStrategy] Starting strategy");
    }
}

impl CommandEmitter for BinanceStrategy {
    fn command_init(&mut self, registry: Arc<CommandRegistry>) {
        info!(
            "[BinanceStrategy] Command channel registered: {:?}",
            registry
        );
        self.command_registry = registry;
    }

    fn command_registry(&self) -> Arc<CommandRegistry> {
        self.command_registry.clone()
    }
}

impl EventHandler for BinanceStrategy {
    async fn on_schedule(&mut self, msg: InfraMsg<AltScheduleEvent>) {
        info!("[BinanceStrategy] AltEventHandler: {:?}", msg);
    }

    /// Start the websocket relay after its task handle is registered.
    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        info!(
            "[BinanceStrategy] Triggering connect for channel: {:?}",
            msg.data.ws_channel
        );
        if let Err(e) = self.connect_channel(&msg.data.ws_channel).await {
            error!("[BinanceStrategy] connect failed: {:?}", e);
        }
    }

    /// Handle Binance candle batches.
    async fn on_candle(&mut self, msg: InfraMsg<Vec<WsCandle>>) {
        info!("[BinanceStrategy] Candle event: {:?}", msg);
    }
}

/// Run the example runtime.
#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    // Binance 1-minute candle stream.
    let binance_ws_candle = WsTaskInfo {
        market: Market::BinanceUmFutures,
        ws_channel: WsChannel::Candles(Some(CandleParam::OneMinute)),
        filter_channels: false, // false for debug msg
        chunk: 1,               // number of websocket connections for this task
        task_base_id: None,
    };

    // Five-second scheduler.
    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(5)),
        chunk: 1,
        task_base_id: None,
    };

    let env = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_alt_event())
        .with_board_cast_channel(BoardCastChannel::default_ws_event())
        // Use *_with_capacity when a stream needs a larger broadcast buffer.
        .with_board_cast_channel(BoardCastChannel::candle_with_capacity(4_096))
        .with_board_cast_channel(BoardCastChannel::default_scheduler())
        .with_task(TaskInfo::WsTask(Arc::new(binance_ws_candle)))
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .with_strategy_module(EmptyStrategy)
        .with_strategy_module(BinanceStrategy::new())
        .build();

    env.execute().await;
}
