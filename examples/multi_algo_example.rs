use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::sleep;
use tracing::{error, info, warn};

use extrema_infra::prelude::*;
use extrema_infra::market_assets::cex::prelude::*;

#[derive(Clone)]
struct EmptyStrategy;

impl CexEventHandler for EmptyStrategy {
    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("[EmptyStrategy] Candle event: {:?}", msg);
    }
}

impl AltEventHandler for EmptyStrategy {
    async fn on_timer(&mut self) {
        println!("[EmptyStrategy] Timer")
    }
}

impl DexEventHandler for EmptyStrategy {}

impl EventHandler for EmptyStrategy {}
impl CommandEmitter for EmptyStrategy {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategy] Command channel initialized");
    }
}

impl Strategy for EmptyStrategy {
    async fn execute(&mut self) {
        info!("[EmptyStrategy] Executing...");
    }
    fn strategy_name(&self) -> &'static str { "EmptyStrategy" }
}

#[derive(Clone)]
struct EmptyStrategyB {
    command_handles: Vec<Arc<CommandHandle>>,
}

impl EmptyStrategyB {
    fn new() -> Self {
        Self {
            command_handles: Vec::new(),
        }
    }
}
impl CexEventHandler for EmptyStrategyB {
    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("[EmptyStrategyB] Candle event: {:?}", msg);
    }
}

impl AltEventHandler for EmptyStrategyB {
    async fn on_timer(&mut self) {
        println!("[EmptyStrategyB] Timer")
    }
}

impl DexEventHandler for EmptyStrategyB {}

impl EventHandler for EmptyStrategyB {}
impl CommandEmitter for EmptyStrategyB {
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategyB] Command channel initialized");
        self.command_handles.push(command_handle);
    }
}

impl Strategy for EmptyStrategyB {
    async fn execute(&mut self) {
        info!("[EmptyStrategyB] Executing...");
    }
    fn strategy_name(&self) -> &'static str { "EmptyStrategyB" }
}


#[derive(Clone)]
struct BinanceStrategy {
    command_handles: Vec<Arc<CommandHandle>>,
    binance_um_cli: BinanceUM,
}

impl BinanceStrategy {
    fn new() -> Self {
        Self {
            command_handles: Vec::new(),
            binance_um_cli: BinanceUM::new(),
        }
    }

    async fn send_subscribe(&self, channel: WsChannel) -> InfraResult<()> {
        let ws_subs = self.generate_ws_subs_msg(channel.clone()).await?;
        info!("[BinanceStrategy] Generating subscribe: {:?}", ws_subs);
        info!("[BinanceStrategy] command_handles count: {}", self.command_handles.len());

        for (idx, handle) in self.command_handles.iter().enumerate() {
            info!("[BinanceStrategy] Sending send_subscribe via handle {}, {:?}", idx, handle);
            let cmd = TaskCommand::Subscribe {
                msg: ws_subs.msg.clone().unwrap(),
                ack: AckHandle::none(),
            };

            info!("[BinanceStrategy] Subscribe command: {:?}", cmd);


            if let Err(e) = handle.cmd_tx.send(cmd).await {
                warn!("[BinanceStrategy] Failed to send subscribe cmd: {:?}", e);
                continue;
            }
        }
        Ok(())
    }

    async fn send_connect(&self, channel: WsChannel) -> InfraResult<()> {
        let ws_subs = self.generate_ws_subs_msg(channel.clone()).await?;
        info!("[BinanceStrategy] Generating connect: {:?}", ws_subs);
        info!("[BinanceStrategy] command_handles count: {}", self.command_handles.len());


        for (idx, handle) in self.command_handles.iter().enumerate() {
            info!("[BinanceStrategy] Sending connect via handle {}, {:?}", idx, handle);
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::Connect {
                msg: ws_subs.url.clone(),
                ack: AckHandle::new(tx),
            };

            info!("[BinanceStrategy] Connect command: {:?}", cmd);
            if let Err(e) = handle.cmd_tx.send(cmd).await {
                warn!("[BinanceStrategy] Failed to send connect cmd: {:?}", e);
                continue;
            }
            // sleep(Duration::from_secs(2)).await;
            // self.send_subscribe(channel.clone()).await?;

            match rx.await {
                Ok(Ok(())) => {
                    info!("[BinanceStrategy] Connected successfully");
                }
                Ok(Err(e)) => error!("[BinanceStrategy] Connect ack failed: {:?}", e),
                Err(_) => warn!("[BinanceStrategy] Connect ack dropped"),
            }
        }
        Ok(())
    }

    async fn generate_ws_subs_msg(&self, channel: WsChannel) -> InfraResult<WsSubscription> {
        info!("[BinanceStrategy] Generating ws subscription message for channel: {:?}", channel);
        let ws_sub = self.binance_um_cli
            .ws_cex_pub_subscription(&channel, &["BTC_USDT_PERP".to_string()])
            .await?;

        info!("[BinanceStrategy] Generated ws subscription: {:?}", ws_sub);
        Ok(ws_sub)
    }
}

impl CexEventHandler for BinanceStrategy {
    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("[BinanceStrategy] Candle event: {:?}", msg);
    }

    async fn on_cex_event(&mut self, _msg: Arc<WsTaskInfo>)  {
        error!("[BinanceStrategy] Received Cex event");
        info!("[BinanceStrategy] Triggering connect for channel: {:?}", _msg);
        self.send_connect(_msg.ws_channel.clone()).await.expect("connect failed");
    }
}

impl AltEventHandler for BinanceStrategy {
    async fn on_timer(&mut self) {
        println!("[BinanceStrategy] Timer event");
    }
}

impl DexEventHandler for BinanceStrategy {}

impl EventHandler for BinanceStrategy { }
impl CommandEmitter for BinanceStrategy {
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        info!("[BinanceStrategy] Command channel registered");
        self.command_handles.push(command_handle);
    }
}

impl Strategy for BinanceStrategy {
    async fn execute(&mut self) {
        info!("[BinanceStrategy] Starting strategy");
        let channel = WsChannel::Candle(Some(CandleParam::OneMinute));
        self.send_connect(channel).await.expect("connect failed");
    }

    // fn strategy_name(&self) -> &'static str { "BinanceStrategy" }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let binance_ws_candle = WsTaskInfo {
        market: Market::BinanceUmFutures,
        ws_channel: WsChannel::Candle(Some(CandleParam::OneMinute)),
        chunk: 1,
    };

    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimerBasedState(2_000),
    };

    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_cex_event())
        .with_board_cast_channel(BoardCastChannel::default_cex_event())
        .with_board_cast_channel(BoardCastChannel::default_candle())
        .with_board_cast_channel(BoardCastChannel::default_candle())
        .with_board_cast_channel(BoardCastChannel::default_timer())
        // .with_strategy(EmptyStrategy)
        .with_strategy(EmptyStrategyB::new())
        .with_strategy(BinanceStrategy::new())
        .with_task(TaskInfo::WsTask(Arc::new(binance_ws_candle)))
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    mediator.execute().await;
}
