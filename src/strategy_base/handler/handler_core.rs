use std::sync::Arc;
use tokio::sync::broadcast;
use futures::future;
use tracing::error;
use crate::market_assets::api_general::OrderParams;
use crate::strategy_base::handler::{
    alt_events::*,
    cex_events::*,
};
use crate::task_execution::{
    task_alt::AltTaskInfo,
    task_ws::WsTaskInfo,
};
use crate::traits::strategy::Strategy;

#[derive(Clone, Debug)]
pub struct InfraMsg<T> {
    pub task_numb: u64,
    pub data: Arc<T>,
}

#[derive(Clone, Debug)]
pub enum BoardCastChannel {
    Alt(broadcast::Sender<InfraMsg<AltTaskInfo>>),
    Cex(broadcast::Sender<InfraMsg<WsTaskInfo>>),
    Dex(broadcast::Sender<InfraMsg<WsTaskInfo>>),
    OrderExecute(broadcast::Sender<InfraMsg<Vec<OrderParams>>>),
    Schedule(broadcast::Sender<InfraMsg<AltScheduleEvent>>),
    Trade(broadcast::Sender<InfraMsg<Vec<WsTrade>>>),
    Lob(broadcast::Sender<InfraMsg<Vec<WsLob>>>),
    Candle(broadcast::Sender<InfraMsg<Vec<WsCandle>>>),
    AccOrder(broadcast::Sender<InfraMsg<Vec<WsAccOrder>>>),
}

impl BoardCastChannel {
    pub fn default_alt_event() -> Self {
        BoardCastChannel::Alt(broadcast::channel(2048).0)
    }

    pub fn default_cex_event() -> Self {
        BoardCastChannel::Cex(broadcast::channel(2048).0)
    }

    pub fn default_dex_event() -> Self {
        BoardCastChannel::Dex(broadcast::channel(2048).0)
    }

    pub fn default_order_execution() -> Self {
        BoardCastChannel::OrderExecute(broadcast::channel(2048).0)
    }

    pub fn default_scheduler() -> Self {
        BoardCastChannel::Schedule(broadcast::channel(2048).0)
    }

    pub fn default_trade() -> Self {
        BoardCastChannel::Trade(broadcast::channel(2048).0)
    }

    pub fn default_lob() -> Self {
        BoardCastChannel::Lob(broadcast::channel(2048).0)
    }

    pub fn default_candle() -> Self {
        BoardCastChannel::Candle(broadcast::channel(2048).0)
    }

    pub fn default_account_order() -> Self {
        BoardCastChannel::AccOrder(broadcast::channel(2048).0)
    }

}

async fn recv_or_pending<T: Clone>(
    rx: &mut Option<broadcast::Receiver<T>>,
) -> Result<T, broadcast::error::RecvError> {
    match rx {
        Some(rx) => rx.recv().await,
        None => future::pending().await,
    }
}

pub(crate) async fn strategy_handler_loop<S>(
    mut strategies: S,
    channels: &Arc<Vec<BoardCastChannel>>
)
where
    S: Strategy,
{
    let mut rx_alt_event = find_alt_event(channels).map(|tx| tx.subscribe());
    let mut rx_cex_event = find_cex_event(channels).map(|tx| tx.subscribe());
    let mut rx_dex_event = find_dex_event(channels).map(|tx| tx.subscribe());

    let mut rx_order_execute = find_order_execution(channels).map(|tx| tx.subscribe());
    let mut rx_schedule = find_scheduler(channels).map(|tx| tx.subscribe());

    let mut rx_trade = find_trade(channels).map(|tx| tx.subscribe());
    let mut rx_lob = find_lob(channels).map(|tx| tx.subscribe());
    let mut rx_candle = find_candle(channels).map(|tx| tx.subscribe());
    
    let mut rx_acc_order = find_acc_order(channels).map(|tx| tx.subscribe());

    loop {
        tokio::select! {
            biased;

            msg = recv_or_pending(&mut rx_trade) => {
                match msg {
                    Ok(msg) => strategies.on_trade(msg).await,
                    Err(e) => {
                        error!("rx_trade err: {:?}, reconnecting...", e);
                        rx_trade = find_trade(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_lob) => {
                match msg {
                    Ok(msg) => strategies.on_lob(msg).await,
                    Err(e) => {
                        error!("rx_lob err: {:?}, reconnecting...", e);
                        rx_lob = find_lob(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_candle) => {
                match msg {
                    Ok(msg) => {
                        strategies.on_candle(msg).await
                    },
                    Err(e) => {
                        error!("rx_candle err: {:?}, reconnecting...", e);
                        rx_candle = find_candle(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_acc_order) => {
                match msg {
                    Ok(msg) => strategies.on_acc_order(msg).await,
                    Err(e) => {
                        error!("rx_acc_order err: {:?}, reconnecting...", e);
                        rx_acc_order = find_acc_order(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_order_execute) => {
                match msg {
                    Ok(msg) => strategies.on_order_execution(msg).await,
                    Err(e) => {
                        error!("rx_order_execute err: {:?}, reconnecting...", e);
                        rx_order_execute = find_order_execution(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_schedule) => {
                match msg {
                    Ok(msg) => strategies.on_schedule(msg).await,
                    Err(e) => {
                        error!("rx_schedule err: {:?}, reconnecting...", e);
                        rx_schedule = find_scheduler(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_alt_event) => {
                match msg {
                    Ok(msg) => {strategies.on_alt_event(msg).await},
                    Err(e) => {
                        error!("rx_alt_event err: {:?}, reconnecting...", e);
                        rx_alt_event = find_alt_event(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_cex_event) => {
                match msg {
                    Ok(msg) => {
                        strategies.on_cex_event(msg).await
                    },
                    Err(e) => {
                        error!("rx_cex_event err: {:?}, reconnecting...", e);
                        rx_cex_event = find_cex_event(channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_dex_event) => {
                match msg {
                    Ok(msg) => strategies.on_dex_event(msg).await,
                    Err(e) => {
                        error!("rx_dex_event err: {:?}, reconnecting...", e);
                        rx_dex_event = find_dex_event(channels).map(|tx| tx.subscribe());
                    },
                };
            },
        }
    }
}

pub(crate) fn find_alt_event(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<AltTaskInfo>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Alt(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_cex_event(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<WsTaskInfo>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Cex(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_dex_event(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<WsTaskInfo>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Dex(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_order_execution(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<Vec<OrderParams>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::OrderExecute(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_scheduler(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<AltScheduleEvent>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Schedule(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_trade(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<Vec<WsTrade>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Trade(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_lob(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<Vec<WsLob>>>> {

    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Lob(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_candle(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<Vec<WsCandle>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Candle(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_acc_order(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<InfraMsg<Vec<WsAccOrder>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::AccOrder(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}





