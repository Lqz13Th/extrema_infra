use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use extrema_infra::{
    arch::market_assets::exchange::prelude::{BinanceUmCli, OkxCli},
    prelude::*,
};

#[derive(Clone)]
struct AccountModule {
    command_handles: Vec<Arc<CommandHandle>>,
    binance_um_cli: BinanceUmCli,
    okx_cli: OkxCli,
}

impl AccountModule {
    pub fn new() -> Self {
        Self {
            command_handles: Vec::new(),
            binance_um_cli: BinanceUmCli::default(),
            okx_cli: OkxCli::default(),
        }
    }

    pub async fn connect_binance_um_acc_channel(&mut self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(channel, 1001) {
            let ws_url = self.binance_um_cli.get_private_connect_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsConnect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsConnect, rx)))
                .await?;
        } else {
            warn!("No handle found for channel {:?}", channel);
        }
        Ok(())
    }

    pub async fn connect_okx_acc_channel(&mut self, channel: &WsChannel) -> InfraResult<()> {
        if let Some(handle) = self.find_ws_handle(channel, 1002) {
            // Step 1: Connect
            let ws_url = self.okx_cli.get_private_connect_msg(channel).await?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsConnect {
                msg: ws_url,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsConnect, rx)))
                .await?;

            // Step 2: Login
            let login_msg = self.okx_cli.ws_login_msg()?;
            let (tx, rx) = oneshot::channel();
            let cmd = TaskCommand::WsMessage {
                msg: login_msg,
                ack: AckHandle::new(tx),
            };
            handle
                .send_command(cmd, Some((AckStatus::WsMessage, rx)))
                .await?;

            // Step 3: Subscribe to private account/order updates
            let ws_msg = self.okx_cli.get_private_sub_msg(channel).await?;
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
        self.binance_um_cli.init_api_key();
        self.okx_cli.init_api_key();
        info!("Init api key");
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
    async fn on_schedule(&mut self, msg: InfraMsg<AltScheduleEvent>) {
        if let Err(e) = self.binance_um_cli.renew_listen_key().await {
            error!("Renew Listen key failed: {:?}", e);
        }

        info!("Renew Binance Listen Key: {:?}", msg);
    }

    async fn on_ws_event(&mut self, msg: InfraMsg<WsTaskInfo>) {
        info!("WS Task Info: {:?}", msg);
        if msg.data.ws_channel == WsChannel::AccountBalAndPos {
            println!("task id: {:?}", msg.data.task_base_id);
            match msg.task_id {
                1001 => {
                    if let Err(e) = self
                        .connect_binance_um_acc_channel(&msg.data.ws_channel)
                        .await
                    {
                        error!(
                            "connect BinanceUm ws private account channel failed: {:?}",
                            e
                        );
                    }
                },
                1002 => {
                    if let Err(e) = self.connect_okx_acc_channel(&msg.data.ws_channel).await {
                        error!("connect okx ws private account channel failed: {:?}", e);
                    }
                },
                _ => {},
            };
        }
    }

    /// Handle account updates from exchange
    async fn on_acc_bal_pos(&mut self, msg: InfraMsg<Vec<WsAccBalPos>>) {
        info!("Updating account status: {:?}", msg);
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Logger initialized");

    let alt_task = AltTaskInfo {
        alt_task_type: AltTaskType::TimeScheduler(Duration::from_secs(1800)),
        chunk: 1,
        task_base_id: None,
    };

    let binance_um_acc_bal_pos_task = WsTaskInfo {
        market: Market::BinanceUmFutures,
        ws_channel: WsChannel::AccountBalAndPos,
        filter_channels: true,
        chunk: 1,
        task_base_id: Some(1001),
    };

    let okx_acc_bal_pos_task = WsTaskInfo {
        market: Market::Okx,
        ws_channel: WsChannel::AccountBalAndPos,
        filter_channels: false,
        chunk: 1,
        task_base_id: Some(1002),
    };

    let env = EnvBuilder::new()
        .with_board_cast_channel(BoardCastChannel::default_alt_event())
        .with_board_cast_channel(BoardCastChannel::default_ws_event())
        .with_board_cast_channel(BoardCastChannel::default_scheduler())
        .with_board_cast_channel(BoardCastChannel::default_account_bal_pos())
        .with_task(TaskInfo::AltTask(Arc::new(alt_task)))
        .with_task(TaskInfo::WsTask(Arc::new(binance_um_acc_bal_pos_task)))
        .with_task(TaskInfo::WsTask(Arc::new(okx_acc_bal_pos_task)))
        .with_strategy_module(AccountModule::new())
        .build();

    env.execute().await;
}
