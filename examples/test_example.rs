use std::sync::Arc;
use serde_json::to_string;
use tokio::sync::{broadcast, oneshot};
use tracing::info;
use extrema_infra::errors::*;
use extrema_infra::traits::strategy::*;
use extrema_infra::infra_core::{
    env_builder::*,
    env_mediator::*,
};
use extrema_infra::market_assets::cex::binance::binance_um_futures_cli::BinanceUM;
use extrema_infra::market_assets::market_core::Market;
use extrema_infra::strategy_base::command::ack_handle::AckHandle;
use extrema_infra::strategy_base::command::command_core::{CommandHandle, TaskCommand};
use extrema_infra::strategy_base::handler::cex_events::WsCandle;
use extrema_infra::strategy_base::handler::handler_core::*;
use extrema_infra::task_execution::alt_register::*;
use extrema_infra::task_execution::alt_register::AltTaskType::TimerBasedState;
use extrema_infra::task_execution::task_general::*;
use extrema_infra::task_execution::ws_register::{CandleParam, WsChannel, WsSubscription, WsTaskInfo};
use extrema_infra::traits::conversion::WsSubscribe;
use extrema_infra::traits::market_cex::CexPublicRest;

// 策略 1
#[derive(Clone)]
struct StrategyA;

impl CexEventHandler for StrategyA {
    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("on_candle msg AAAAAAAAAAAAAAAAAAAAAA: {:?}", msg);
    }
}

impl AltEventHandler for StrategyA {}

impl EventHandler for StrategyA {}
impl CommandEmitter for StrategyA {
    fn command_init(&mut self, _command_handle: CommandHandle) {
        info!("CommandInit StrategyA");
    }
}

impl Strategy for StrategyA {
    async fn execute(&mut self) {
        info!("execute StrategyA");
    }
    fn name(&self) -> &'static str { "StrategyA" }
}

// 策略 2
#[derive(Clone)]
struct StrategyB;

impl CexEventHandler for StrategyB {}

impl AltEventHandler for StrategyB {}

impl EventHandler for StrategyB {}
impl CommandEmitter for StrategyB {
    fn command_init(&mut self, _command_handle: CommandHandle) {
        info!("command-init StrategyB");
    }
}

impl Strategy for StrategyB {
    async fn execute(&mut self) {
        info!("execute StrategyB");
    }

    fn name(&self) -> &'static str { "StrategyB" }
}

#[derive(Clone)]
struct StrategyC {
    command_handles: Vec<CommandHandle>,
    binance_um_cli: Arc<BinanceUM>,
}

impl StrategyC {
    fn new() -> Self {
        Self {
            command_handles: Vec::new(),
            binance_um_cli: Arc::new(BinanceUM::new()),
        }
    }

    pub async fn send_subscribe(&self) -> InfraResult<()> {
        let test_channel = WsChannel::Candle(Some(CandleParam::OneMinute));

        let ws_subs = self.generate_ws_subs_msg(test_channel).await?;

        for handle in &self.command_handles {
            let (tx, rx) = oneshot::channel();
            let ack = AckHandle::new(tx);
            let cmd = TaskCommand::Subscribe {
                msg: ws_subs.msg.clone().unwrap(),
                ack,
            };

            if let Err(e) = handle.cmd_tx.send(cmd).await {
                tracing::warn!("Failed to send subscribe command: {:?}", e);
                continue;
            }

            match rx.await {
                Ok(res) => {
                    if let Err(e) = res {
                        tracing::error!("Subscribe ack failed: {:?}", e);
                    }
                }
                Err(_) => {
                    tracing::warn!("Subscribe ack receiver dropped");
                }
            }

        }

        Ok(())
    }

    pub async fn send_connect(&self, ws_channel: WsChannel) -> InfraResult<()> {
        let ws_subs = self.generate_ws_subs_msg(ws_channel).await?;

        for handle in &self.command_handles {
            let (tx, rx) = oneshot::channel();
            let ack = AckHandle::new(tx);
            let cmd = TaskCommand::Connect {
                msg: ws_subs.url.clone(),
                ack,
            };

            if let Err(e) = handle.cmd_tx.send(cmd).await {
                tracing::warn!("Failed to send subscribe command: {:?}", e);
                continue;
            }

            match rx.await {
                Ok(res) => {
                    info!("connected");
                    self.send_subscribe().await?;
                    if let Err(e) = res {
                        tracing::error!("Subscribe ack failed: {:?}", e);
                    }
                }
                Err(_) => {
                    tracing::warn!("Subscribe ack receiver dropped");
                }
            }

        }

        Ok(())
    }

    pub async fn generate_ws_subs_msg(&self, ws_channel: WsChannel) -> InfraResult<WsSubscription> {
        // let symbols = self.binance_um_cli.get_live_symbols().await?;
        // println!("symbols: {:?}", symbols);
        let ws_subs = self.binance_um_cli.ws_cex_pub_subscription(&ws_channel, &["BTC_USDT_PERP".to_string()]).await?;
        Ok(ws_subs)
    }
}

impl CexEventHandler for StrategyC {
    async fn on_candle(&mut self, msg: Arc<Vec<WsCandle>>) {
        info!("on_candle msg: {:?}", msg);
    }
}

impl AltEventHandler for StrategyC {}

impl EventHandler for StrategyC {
    async fn event_init(&mut self, task_info: Arc<TaskInfo>) {
        info!("event_init task info: {:?}", task_info);
        self.send_subscribe().await.unwrap();
    }
}
impl CommandEmitter for StrategyC {
    fn command_init(&mut self, command_handle: CommandHandle) {
        info!("CommandInit StrategyC");
        info!("command_handle C: {:?}", command_handle);
        self.command_handles.push(command_handle);
    }
}

impl Strategy for StrategyC {
    async fn execute(&mut self) {
        info!("execute strategyC");
        let test_channel = WsChannel::Candle(Some(CandleParam::OneMinute));
        self.send_connect(test_channel).await.expect("bug");
    }

    fn name(&self) -> &'static str { "StrategyC" }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let binance_ws_candle = WsTaskInfo {
        market: Market::BinanceUmFutures,
        channel: WsChannel::Candle(Some(CandleParam::OneMinute)),
        chunk: 1,
    };

    let alt_task = AltTaskInfo {
        alt_task_type: TimerBasedState(5_000),
    };

    let mediator = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_timer())
        .with_board_cast_channel(BoardCastChannel::default_candle())
        .with_strategy(StrategyA)
        // .with_strategy(StrategyB)
        .with_strategy(StrategyC::new())
        .with_task(TaskInfo::WsTask(Arc::new(binance_ws_candle)))
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .build();

    mediator.initialize().await.execute().await;
}
