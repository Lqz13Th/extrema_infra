//! Common imports for strategy binaries.
//!
//! Most applications can start with:
//!
//! ```rust
//! use extrema_infra::prelude::*;
//! ```
//!
//! The prelude contains the runtime builder, strategy traits, task descriptors,
//! broadcast channel types, event masks, command handles, normalized market
//! data structures, and shared error/result aliases. Exchange-specific client
//! structs remain available under `arch::market_assets::exchange::prelude` when
//! the matching feature is enabled.
pub use crate::errors::{InfraError, InfraResult};

pub use crate::arch::{
    infra_core::{env_builder::EnvBuilder, env_mediator::EnvMediator},
    market_assets::{
        base_data::*,
        market_core::{Market, MarketScope},
    },
    strategy_base::{
        command::{
            ack_handle::{AckHandle, AckStatus},
            command_core::*,
        },
        handler::{alt_events::*, event_mask::*, handler_core::*, lob_events::*},
    },
    task_execution::{task_alt::*, task_general::TaskInfo, task_ws::*},
    traits::{conversion::*, market_lob::*, strategy::*},
};
