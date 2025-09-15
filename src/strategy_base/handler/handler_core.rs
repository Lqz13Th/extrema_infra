use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::error;
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
pub enum BoardCastChannel {
    Alt(broadcast::Sender<Arc<AltTaskInfo>>),
    Cex(broadcast::Sender<Arc<WsTaskInfo>>),
    Dex(broadcast::Sender<Arc<WsTaskInfo>>),
    Timer(broadcast::Sender<()>),
    Candle(broadcast::Sender<Arc<Vec<WsCandle>>>),
    Trade(broadcast::Sender<Arc<Vec<WsTrade>>>),
    Lob(broadcast::Sender<Arc<Vec<WsLob>>>),
}

impl BoardCastChannel {
    pub fn default_alt_event() -> Self {
        BoardCastChannel::Alt(broadcast::channel(1024).0)
    }

    pub fn default_cex_event() -> Self {
        BoardCastChannel::Cex(broadcast::channel(1024).0)
    }

    pub fn default_dex_event() -> Self {
        BoardCastChannel::Dex(broadcast::channel(1024).0)
    }

    pub fn default_timer() -> Self {
        BoardCastChannel::Timer(broadcast::channel(1024).0)
    }

    pub fn default_trade() -> Self {
        BoardCastChannel::Trade(broadcast::channel(1024).0)
    }

    pub fn default_lob() -> Self {
        BoardCastChannel::Lob(broadcast::channel(1024).0)
    }

    pub fn default_candle() -> Self {
        BoardCastChannel::Candle(broadcast::channel(1024).0)
    }
}


async fn recv_or_pending<T: Clone>(
    rx: &mut Option<broadcast::Receiver<T>>,
) -> Result<T, broadcast::error::RecvError> {
    match rx {
        Some(rx) => rx.recv().await,
        None => futures::future::pending().await,
    }
}

pub(crate) async fn strategy_handler_loop<S>(
    mut strategies: S,
    channels: &Arc<Vec<BoardCastChannel>>
)
where
    S: Strategy,
{
    println!("channels: {:?}", channels);
    let mut rx_alt_event = find_alt_event(&channels).map(|tx| tx.subscribe());
    let mut rx_cex_event = find_cex_event(&channels).map(|tx| tx.subscribe());
    let mut rx_dex_event = find_dex_event(&channels).map(|tx| tx.subscribe());

    let mut rx_timer = find_timer(&channels).map(|tx| tx.subscribe());

    let mut rx_trade = find_trade(&channels).map(|tx| tx.subscribe());
    let mut rx_lob = find_lob(&channels).map(|tx| tx.subscribe());
    let mut rx_candle = find_candle(&channels).map(|tx| tx.subscribe());

    loop {
        tokio::select! {
            msg = recv_or_pending(&mut rx_trade) => {
                match msg {
                    Ok(msg) => strategies.on_trade(msg).await,
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("rx_lob closed, reconnecting...");
                        rx_trade = find_trade(&channels).map(|tx| tx.subscribe());
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("rx_lob lagged, skipped {} messages", n);
                    },
                };
            },
            msg = recv_or_pending(&mut rx_lob) => {
                match msg {
                    Ok(msg) => strategies.on_lob(msg).await,
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("rx_lob closed, reconnecting...");
                        rx_lob = find_lob(&channels).map(|tx| tx.subscribe());
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("rx_lob lagged, skipped {} messages", n);
                    },
                };
            },
            msg = recv_or_pending(&mut rx_candle) => {
                match msg {
                    Ok(msg) => {
                        error!("msg cansdlelelelel{:?}", msg.clone());
                        strategies.on_candle(msg).await
                    },
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("rx_candle closed, reconnecting...");
                        rx_candle = find_candle(&channels).map(|tx| tx.subscribe());
                    },
                    Err(e) => {
                        tracing::warn!("rx_candle err: {:?}", e);
                    },
                };
            },
            msg = recv_or_pending(&mut rx_timer) => {
                match msg {
                    Ok(_msg) => strategies.on_timer().await,
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("rx_timer closed, reconnecting...");
                        rx_timer = find_timer(&channels).map(|tx| tx.subscribe());
                    },
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                    },
                };
            },
            msg = recv_or_pending(&mut rx_alt_event) => {
                match msg {
                    Ok(msg) => {strategies.on_alt_event(msg).await},
                    Err(_) => {
                        rx_alt_event = find_alt_event(&channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_cex_event) => {
                match msg {
                    Ok(msg) => {
                        println!("rx_cex event: {:?}", msg);
                        strategies.on_cex_event(msg).await
                    },
                    Err(e) => {
                        println!("error: {:?}", e);
                        rx_cex_event = find_cex_event(&channels).map(|tx| tx.subscribe());
                    },
                };
            },
            msg = recv_or_pending(&mut rx_dex_event) => {
                match msg {
                    Ok(msg) => strategies.on_dex_event(msg).await,
                    Err(_) => {
                        rx_dex_event = find_dex_event(&channels).map(|tx| tx.subscribe());
                    },
                };
            },
        }
    }
}



pub(crate) fn find_candle(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<Arc<Vec<WsCandle>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Candle(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_trade(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<Arc<Vec<WsTrade>>>> {
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
) -> Option<broadcast::Sender<Arc<Vec<WsLob>>>> {

    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Lob(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_timer(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<()>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Timer(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_alt_event(
    channels: &Arc<Vec<BoardCastChannel>>
) -> Option<broadcast::Sender<Arc<AltTaskInfo>>> {
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
) -> Option<broadcast::Sender<Arc<WsTaskInfo>>> {
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
) -> Option<broadcast::Sender<Arc<WsTaskInfo>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Dex(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}





