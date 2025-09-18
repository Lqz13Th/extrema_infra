use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use extrema_infra::prelude::*;

#[derive(Clone)]
struct EmptyStrategy;
impl Strategy for EmptyStrategy {
    async fn execute(&mut self) {
        info!("[EmptyStrategy] Executing...");
    }
    fn strategy_name(&self) -> &'static str { "EmptyStrategy" }
}
impl EventHandler for EmptyStrategy {}
impl AltEventHandler for EmptyStrategy {
    async fn on_timer(
        &mut self,
        msg: Arc<AltTimerEvent>,
    ) {
        info!("[EmptyStrategy] AltEventHandler: {:?}", msg);
    }
}
impl CexEventHandler for EmptyStrategy {
    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("[EmptyStrategy] Candle event: {:?}", msg);
    }
}

impl DexEventHandler for EmptyStrategy {}

impl CommandEmitter for EmptyStrategy {
    fn command_init(&mut self, _command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategy] Command channel initialized");
    }
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

impl Strategy for EmptyStrategyB {
    async fn execute(&mut self) {
        info!("[EmptyStrategyB] Executing...");
    }
    fn strategy_name(&self) -> &'static str { "EmptyStrategyB" }
}
impl EventHandler for EmptyStrategyB {}

impl AltEventHandler for EmptyStrategyB {
    async fn on_timer(
        &mut self,
        msg: Arc<AltTimerEvent>,
    ) {
        info!("[EmptyStrategyB] AltEventHandler: {:?}", msg);
    }
}

impl CexEventHandler for EmptyStrategyB {
    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("[EmptyStrategyB] Candle event: {:?}", msg);
    }
}

impl DexEventHandler for EmptyStrategyB {}


impl CommandEmitter for EmptyStrategyB {
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        info!("[EmptyStrategyB] Command channel initialized");
        self.command_handles.push(command_handle);
    }
}




#[derive(Clone)]
struct BinanceStrategy {
    command_handles: Vec<Arc<CommandHandle>>,
    binance_um_cli: BinanceUmCli,
}

impl BinanceStrategy {
    fn new() -> Self {
        Self {
            command_handles: Vec::new(),
            binance_um_cli: BinanceUmCli::default(),
        }
    }

    async fn send_subscribe(&self, channel: WsChannel) -> InfraResult<()> {
        let ws_subs = self.generate_ws_subs_msg(channel.clone()).await?;
        for (idx, handle) in self.command_handles.iter().enumerate() {
            info!("[BinanceStrategy] Sending send_subscribe via handle {}, {:?}", idx, handle);
            let cmd = TaskCommand::Subscribe {
                msg: ws_subs.msg.clone().unwrap(),
                ack: AckHandle::none(),
            };

            if let Err(e) = handle.cmd_tx.send(cmd).await {
                warn!("[BinanceStrategy] Failed to send subscribe cmd: {:?}", e);
                continue;
            }
        }
        Ok(())
    }

    async fn send_connect(&self, channel: WsChannel) -> InfraResult<()> {
        let ws_subs = self.generate_ws_subs_msg(channel.clone()).await?;
        for (idx, handle) in self.command_handles.iter().enumerate() {
            info!("[BinanceStrategy] Sending connect via handle {}, {:?}", idx, handle);
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::Connect {
                msg: ws_subs.url.clone(),
                ack: AckHandle::new(tx),
            };

            if let Err(e) = handle.cmd_tx.send(cmd).await {
                warn!("[BinanceStrategy] Failed to send connect cmd: {:?}", e);
                continue;
            }

            match rx.await {
                Ok(Ok(())) => {
                    self.send_subscribe(channel.clone()).await?;
                },
                Ok(Err(e)) => error!("[BinanceStrategy] Connect ack failed: {:?}", e),
                Err(_) => warn!("[BinanceStrategy] Connect ack dropped"),
            }
        }

        Ok(())
    }

    async fn generate_ws_subs_msg(&self, channel: WsChannel) -> InfraResult<WsSubscription> {
        let ws_sub = self.binance_um_cli
            .ws_cex_pub_subscription(&channel, &["BTC_USDT_PERP".to_string()])
            .await?;

        Ok(ws_sub)
    }
}

impl Strategy for BinanceStrategy {
    async fn execute(&mut self) {
        info!("[BinanceStrategy] Starting strategy");
    }
}
impl EventHandler for BinanceStrategy {}

impl AltEventHandler for BinanceStrategy {
    async fn on_timer(
        &mut self,
        msg: Arc<AltTimerEvent>,
    ) {
        info!("[BinanceStrategy] AltEventHandler: {:?}", msg);
    }
}

impl CexEventHandler for BinanceStrategy {
    async fn on_cex_event(&mut self, _msg: Arc<WsTaskInfo>)  {
        info!("[BinanceStrategy] Triggering connect for channel: {:?}", _msg.ws_channel);
        self.send_connect(_msg.ws_channel.clone()).await.expect("connect failed");
    }

    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("[BinanceStrategy] Candle event: {:?}", msg);
    }
}
impl DexEventHandler for BinanceStrategy {}

impl CommandEmitter for BinanceStrategy {
    fn command_init(&mut self, command_handle: Arc<CommandHandle>) {
        info!("[BinanceStrategy] Command channel registered");
        self.command_handles.push(command_handle);
    }
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
        alt_task_type: AltTaskType::TimerBasedState(5),
    };

    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_cex_event())
        .with_board_cast_channel(BoardCastChannel::default_candle())
        .with_board_cast_channel(BoardCastChannel::default_candle()) // duplicated skip
        .with_board_cast_channel(BoardCastChannel::default_timer())
        .with_strategy(EmptyStrategy)
        .with_strategy(EmptyStrategyB::new())
        .with_strategy(BinanceStrategy::new())
        .with_task(TaskInfo::WsTask(Arc::new(binance_ws_candle)))
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    mediator.execute().await;
}
