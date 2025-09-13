use std::sync::Arc;
use tokio::sync::broadcast;

use crate::traits::strategy::Strategy;
use crate::strategy_base::event_notify::{
    alt_notify::*,
    cex_notify::*,
};

#[derive(Clone, Debug)]
pub enum BoardCastChannel {
    Trade(broadcast::Sender<Arc<Vec<WsTrade>>>),
    Lob(broadcast::Sender<Arc<Vec<WsLob>>>),
    Timer(broadcast::Sender<()>),
}

impl BoardCastChannel {
    pub fn default_trade() -> Self {
        BoardCastChannel::Trade(broadcast::channel(1024).0)
    }

    pub fn default_lob() -> Self {
        BoardCastChannel::Lob(broadcast::channel(1024).0)
    }

    pub fn default_timer() -> Self {
        BoardCastChannel::Timer(broadcast::channel(16).0)
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

pub(crate) async fn strategy_board_cast_loop<S>(
    mut strategies: S,
    channels: Arc<Vec<BoardCastChannel>>,
)
where
    S: Strategy + Clone,
{
    let mut rx_trade = find_trade(&channels).map(|tx| tx.subscribe());
    let mut rx_lob = find_lob(&channels).map(|tx| tx.subscribe());
    let mut rx_timer = find_timer(&channels).map(|tx| tx.subscribe());


    loop {
        tokio::select! {
            msg = recv_or_pending(&mut rx_trade) => {
                match msg {
                    Ok(msg) => strategies.on_trade(msg).await,
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::error!("rx_trade closed, reconnecting...");
                        rx_trade = find_trade(&channels).map(|tx| tx.subscribe());
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("rx_trade lagged, skipped {} messages", n);
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
        }
    }
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






