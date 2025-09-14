use std::sync::Arc;

use crate::strategy_base::{
    handler::handler_core::BoardCastChannel,
};

#[derive(Clone, Debug)]
pub(crate) struct EnvCore<S> {
    pub(crate) strategy: S,
    pub(crate) channel: Arc<Vec<BoardCastChannel>>,
}
