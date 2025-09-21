use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::sleep;
use tracing::{error, info, warn};

use extrema_infra::prelude::*;


///# Empty strategy
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
    async fn on_schedule(
        &mut self,
        msg: InfraMsg<AltScheduleEvent>,
    ) {
        info!("[EmptyStrategy] AltEventHandler: {:?}", msg);
    }
}
impl CexEventHandler for EmptyStrategy {
    async fn on_candle(&mut self, msg: InfraMsg<Vec<WsCandle>>) {
        info!("[EmptyStrategy] Candle event: {:?}", msg);
    }
}

impl CommandEmitter for EmptyStrategy {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategy] Command channel registered: {:?}", _command_handle);
    }

    fn command_registry(&self) -> Vec<Arc<CommandHandle>> {
        Vec::new()
    }
}


///# Binance strategy
#[derive(Clone)]
struct BinanceStrategy {
    command_handles: Vec<Arc<CommandHandle>>,
    binance_um_cli: BinanceUmCli, // public binance um future client without api keys
}

impl BinanceStrategy {
    fn new() -> Self {
        Self {
            command_handles: Vec::new(),
            binance_um_cli: BinanceUmCli::default(),
        }
    }

    async fn connect_channel(&self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(&channel, 1) {
            info!("[BinanceStrategy] Sending connect to {:?}", handle);

            // connect websocket channel
            let ws_url = self.binance_um_cli.get_public_connect_msg(&channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::Connect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle.send_command(cmd, Some((AckStatus::Connect, rx))).await?;

            // send subscribe message
            let ws_msg = self.binance_um_cli
                .get_public_sub_msg(&channel, Some(&["BTC_USDT_PERP".to_string()]))
                .await?;

            let cmd = TaskCommand::Subscribe {
                msg: ws_msg,
                ack: AckHandle::none(),
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
    async fn on_cex_event(&mut self, msg: InfraMsg<WsTaskInfo>)  {
        info!("[BinanceStrategy] Triggering connect for channel: {:?}", msg.data.ws_channel);
        if let Err(e) = self.connect_channel(&msg.data.ws_channel).await {
            error!("[BinanceStrategy] connect failed: {:?}", e);
        }
    }

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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let binance_ws_candle = WsTaskInfo {
        market: Market::BinanceUmFutures,
        ws_channel: WsChannel::Candle(Some(CandleParam::OneMinute)),
        chunk: 1, // how many websocket connection on each task
    };

    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimerBasedState(5),
        chunk: 1,
    };

    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_cex_event())
        .with_board_cast_channel(BoardCastChannel::default_candle())
        .with_board_cast_channel(BoardCastChannel::default_candle()) // duplicated skip
        .with_board_cast_channel(BoardCastChannel::default_schedule())
        .with_strategy_module(EmptyStrategy)
        .with_strategy_module(BinanceStrategy::new())
        .with_task(TaskInfo::WsTask(Arc::new(binance_ws_candle)))
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    mediator.execute().await;
}
