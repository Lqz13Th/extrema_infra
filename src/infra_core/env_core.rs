use std::sync::Arc;

use crate::strategy_base::event_notify::{
    board_cast_channels::*,
};

#[derive(Clone, Debug)]
pub(crate) struct EnvCore<S> {
    pub(crate) strategies: S,
    pub(crate) board_cast_channels: Arc<Vec<BoardCastChannel>>,
}
