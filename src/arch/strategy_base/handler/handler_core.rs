use futures::future;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::error;

use crate::arch::{
    strategy_base::handler::{alt_events::*, event_mask::EventMask, lob_events::*},
    task_execution::{task_alt::AltTaskInfo, task_ws::WsTaskInfo},
    traits::strategy::Strategy,
};

/// Message envelope published through runtime broadcast channels.
///
/// `task_id` identifies the task instance that produced the event. `data` is
/// reference-counted so every interested strategy module can receive the same
/// payload without copying the full event body.
#[derive(Clone, Debug)]
pub struct InfraMsg<T> {
    /// Runtime task id that emitted this message.
    pub task_id: u64,
    /// Shared event payload.
    pub data: Arc<T>,
}

/// Broadcast event streams available inside an environment.
///
/// Add the variants a process needs with
/// [`EnvBuilder::with_board_cast_channel`]. Each variant maps to one callback
/// on [`EventHandler`]. Strategy modules can narrow which registered channels
/// they subscribe to by overriding
/// [`EventHandler::event_mask`](crate::arch::traits::strategy::EventHandler::event_mask).
///
/// [`EnvBuilder::with_board_cast_channel`]: crate::arch::infra_core::env_builder::EnvBuilder::with_board_cast_channel
/// [`EventHandler`]: crate::arch::traits::strategy::EventHandler
#[derive(Clone, Debug)]
pub enum BoardCastChannel {
    /// Generic alt-task lifecycle/control events.
    Alt(broadcast::Sender<InfraMsg<AltTaskInfo>>),
    /// Generic websocket-task lifecycle/control events.
    Ws(broadcast::Sender<InfraMsg<WsTaskInfo>>),
    /// Order execution batches.
    OrderExecute(broadcast::Sender<InfraMsg<Vec<AltOrder>>>),
    /// Instrument or portfolio target intents.
    InstIntent(broadcast::Sender<InfraMsg<AltIntent>>),
    /// Model prediction tensors.
    ModelPreds(broadcast::Sender<InfraMsg<AltTensor>>),
    /// Periodic scheduler ticks.
    Schedule(broadcast::Sender<InfraMsg<AltScheduleEvent>>),
    /// Public trade batches.
    Trade(broadcast::Sender<InfraMsg<Vec<WsTrade>>>),
    /// Public order book updates.
    Lob(broadcast::Sender<InfraMsg<Vec<WsLob>>>),
    /// Public candle batches.
    Candle(broadcast::Sender<InfraMsg<Vec<WsCandle>>>),
    /// Private account order updates.
    AccOrder(broadcast::Sender<InfraMsg<Vec<WsAccOrder>>>),
    /// Private account balance and position updates.
    AccBalPos(broadcast::Sender<InfraMsg<Vec<WsAccBalPos>>>),
    /// Private account position-only updates.
    AccPos(broadcast::Sender<InfraMsg<Vec<WsAccPosition>>>),
}

impl BoardCastChannel {
    /// Creates the default generic alt-task event channel.
    pub fn default_alt_event() -> Self {
        BoardCastChannel::Alt(broadcast::channel(2048).0)
    }

    /// Creates the default generic websocket-task event channel.
    pub fn default_ws_event() -> Self {
        BoardCastChannel::Ws(broadcast::channel(2048).0)
    }

    /// Creates the default order execution channel.
    pub fn default_order_execution() -> Self {
        BoardCastChannel::OrderExecute(broadcast::channel(2048).0)
    }

    /// Creates the default instrument intent channel.
    pub fn default_inst_intent() -> Self {
        BoardCastChannel::InstIntent(broadcast::channel(2048).0)
    }

    /// Creates the default model prediction channel.
    pub fn default_model_preds() -> Self {
        BoardCastChannel::ModelPreds(broadcast::channel(2048).0)
    }

    /// Creates the default scheduler channel.
    pub fn default_scheduler() -> Self {
        BoardCastChannel::Schedule(broadcast::channel(2048).0)
    }

    /// Creates the default public trade channel.
    pub fn default_trade() -> Self {
        BoardCastChannel::Trade(broadcast::channel(2048).0)
    }

    /// Creates the default public order book channel.
    pub fn default_lob() -> Self {
        BoardCastChannel::Lob(broadcast::channel(2048).0)
    }

    /// Creates the default public candle channel.
    pub fn default_candle() -> Self {
        BoardCastChannel::Candle(broadcast::channel(2048).0)
    }

    /// Creates the default private account order channel.
    pub fn default_account_order() -> Self {
        BoardCastChannel::AccOrder(broadcast::channel(2048).0)
    }

    /// Creates the default private balance/position channel.
    pub fn default_account_bal_pos() -> Self {
        BoardCastChannel::AccBalPos(broadcast::channel(2048).0)
    }

    /// Creates the default private account position channel.
    pub fn default_account_pos() -> Self {
        BoardCastChannel::AccPos(broadcast::channel(2048).0)
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

fn subscribe_if<T, Find>(
    mask: EventMask,
    event: EventMask,
    find_sender: Find,
) -> Option<broadcast::Receiver<T>>
where
    T: Clone,
    Find: FnOnce() -> Option<broadcast::Sender<T>>,
{
    if mask.contains(event) {
        find_sender().map(|tx| tx.subscribe())
    } else {
        None
    }
}

pub(crate) async fn strategy_handler_loop<S>(
    mut strategies: S,
    channels: &Arc<Vec<BoardCastChannel>>,
) where
    S: Strategy,
{
    let event_mask = strategies.event_mask();

    // Event init
    let mut rx_alt_event = subscribe_if(event_mask, EventMask::ALT_EVENT, || {
        find_alt_event(channels)
    });
    let mut rx_ws_event = subscribe_if(event_mask, EventMask::WS_EVENT, || find_ws_event(channels));

    // Alt event
    let mut rx_order_execute = subscribe_if(event_mask, EventMask::ORDER_EXECUTION, || {
        find_order_execution(channels)
    });
    let mut rx_inst_intent = subscribe_if(event_mask, EventMask::INST_INTENT, || {
        find_inst_intent(channels)
    });
    let mut rx_preds = subscribe_if(event_mask, EventMask::MODEL_PREDS, || {
        find_model_preds(channels)
    });
    let mut rx_schedule =
        subscribe_if(event_mask, EventMask::SCHEDULE, || find_scheduler(channels));

    // Ws pub event
    let mut rx_trade = subscribe_if(event_mask, EventMask::TRADE, || find_trade(channels));
    let mut rx_lob = subscribe_if(event_mask, EventMask::LOB, || find_lob(channels));
    let mut rx_candle = subscribe_if(event_mask, EventMask::CANDLE, || find_candle(channels));

    // Ws pri event
    let mut rx_acc_order = subscribe_if(event_mask, EventMask::ACC_ORDER, || {
        find_acc_order(channels)
    });
    let mut rx_acc_bal_pos = subscribe_if(event_mask, EventMask::ACC_BAL_POS, || {
        find_acc_bal_pos(channels)
    });
    let mut rx_acc_pos = subscribe_if(event_mask, EventMask::ACC_POS, || find_acc_pos(channels));

    loop {
        tokio::select! {
            biased;

            msg = recv_or_pending(&mut rx_trade) => {
                match msg {
                    Ok(msg) => strategies.on_trade(msg).await,
                    Err(e) => {
                        error!("rx_trade err: {:?}, reconnecting...", e);
                        rx_trade =
                            subscribe_if(event_mask, EventMask::TRADE, || find_trade(channels));
                    },
                };
            },
            msg = recv_or_pending(&mut rx_lob) => {
                match msg {
                    Ok(msg) => strategies.on_lob(msg).await,
                    Err(e) => {
                        error!("rx_lob err: {:?}, reconnecting...", e);
                        rx_lob = subscribe_if(event_mask, EventMask::LOB, || find_lob(channels));
                    },
                };
            },
            msg = recv_or_pending(&mut rx_candle) => {
                match msg {
                    Ok(msg) => strategies.on_candle(msg).await,
                    Err(e) => {
                        error!("rx_candle err: {:?}, reconnecting...", e);
                        rx_candle =
                            subscribe_if(event_mask, EventMask::CANDLE, || find_candle(channels));
                    },
                };
            },
            msg = recv_or_pending(&mut rx_acc_order) => {
                match msg {
                    Ok(msg) => strategies.on_acc_order(msg).await,
                    Err(e) => {
                        error!("rx_acc_order err: {:?}, reconnecting...", e);
                        rx_acc_order = subscribe_if(event_mask, EventMask::ACC_ORDER, || {
                            find_acc_order(channels)
                        });
                    },
                };
            },
            msg = recv_or_pending(&mut rx_acc_bal_pos) => {
                match msg {
                    Ok(msg) => strategies.on_acc_bal_pos(msg).await,
                    Err(e) => {
                        error!("rx_acc_bal_pos err: {:?}, reconnecting...", e);
                        rx_acc_bal_pos = subscribe_if(event_mask, EventMask::ACC_BAL_POS, || {
                            find_acc_bal_pos(channels)
                        });
                    },
                };
            },
            msg = recv_or_pending(&mut rx_acc_pos) => {
                match msg {
                    Ok(msg) => strategies.on_acc_pos(msg).await,
                    Err(e) => {
                        error!("rx_acc_pos err: {:?}, reconnecting...", e);
                        rx_acc_pos =
                            subscribe_if(event_mask, EventMask::ACC_POS, || find_acc_pos(channels));
                    },
                };
            },
            msg = recv_or_pending(&mut rx_order_execute) => {
                match msg {
                    Ok(msg) => strategies.on_order_execution(msg).await,
                    Err(e) => {
                        error!("rx_order_execute err: {:?}, reconnecting...", e);
                        rx_order_execute =
                            subscribe_if(event_mask, EventMask::ORDER_EXECUTION, || {
                                find_order_execution(channels)
                            });
                    },
                };
            },
            msg = recv_or_pending(&mut rx_inst_intent) => {
                match msg {
                    Ok(msg) => strategies.on_inst_intent(msg).await,
                    Err(e) => {
                        error!("rx_inst_intent err: {:?}, reconnecting...", e);
                        rx_inst_intent = subscribe_if(event_mask, EventMask::INST_INTENT, || {
                            find_inst_intent(channels)
                        });
                    },
                };
            },
             msg = recv_or_pending(&mut rx_preds) => {
                match msg {
                    Ok(msg) => strategies.on_preds(msg).await,
                    Err(e) => {
                        error!("rx_preds err: {:?}, reconnecting...", e);
                        rx_preds = subscribe_if(event_mask, EventMask::MODEL_PREDS, || {
                            find_model_preds(channels)
                        });
                    },
                };
            },
            msg = recv_or_pending(&mut rx_schedule) => {
                match msg {
                    Ok(msg) => strategies.on_schedule(msg).await,
                    Err(e) => {
                        error!("rx_schedule err: {:?}, reconnecting...", e);
                        rx_schedule = subscribe_if(event_mask, EventMask::SCHEDULE, || {
                            find_scheduler(channels)
                        });
                    },
                };
            },
            msg = recv_or_pending(&mut rx_alt_event) => {
                match msg {
                    Ok(msg) => strategies.on_alt_event(msg).await,
                    Err(e) => {
                        error!("rx_alt_event err: {:?}, reconnecting...", e);
                        rx_alt_event = subscribe_if(event_mask, EventMask::ALT_EVENT, || {
                            find_alt_event(channels)
                        });
                    },
                };
            },
            msg = recv_or_pending(&mut rx_ws_event) => {
                match msg {
                    Ok(msg) => strategies.on_ws_event(msg).await,
                    Err(e) => {
                        error!("rx_ws_event err: {:?}, reconnecting...", e);
                        rx_ws_event = subscribe_if(event_mask, EventMask::WS_EVENT, || {
                            find_ws_event(channels)
                        });
                    },
                };
            },
        }
    }
}

pub(crate) fn find_alt_event(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<AltTaskInfo>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Alt(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_ws_event(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<WsTaskInfo>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Ws(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_order_execution(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<Vec<AltOrder>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::OrderExecute(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_inst_intent(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<AltIntent>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::InstIntent(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_scheduler(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<AltScheduleEvent>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::Schedule(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_model_preds(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<AltTensor>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::ModelPreds(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_trade(
    channels: &Arc<Vec<BoardCastChannel>>,
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
    channels: &Arc<Vec<BoardCastChannel>>,
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
    channels: &Arc<Vec<BoardCastChannel>>,
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
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<Vec<WsAccOrder>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::AccOrder(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_acc_bal_pos(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<Vec<WsAccBalPos>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::AccBalPos(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}

pub(crate) fn find_acc_pos(
    channels: &Arc<Vec<BoardCastChannel>>,
) -> Option<broadcast::Sender<InfraMsg<Vec<WsAccPosition>>>> {
    channels.iter().find_map(|ch| {
        if let BoardCastChannel::AccPos(tx) = ch {
            Some(tx.clone())
        } else {
            None
        }
    })
}
