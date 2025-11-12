use std::sync::Arc;
use crate::arch::strategy_base::handler::handler_core::BoardCastChannel;

#[derive(Clone, Debug)]
pub(crate) struct EnvCore<S> {
    pub strategy: S,
    pub channel: Arc<Vec<BoardCastChannel>>,
}
