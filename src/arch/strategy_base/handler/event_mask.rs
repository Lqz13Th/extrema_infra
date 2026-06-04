/// Bitmask describing which runtime event streams a strategy module consumes.
///
/// Strategies default to [`EventMask::ALL`] for backwards compatibility. A
/// latency-sensitive module can override
/// [`EventHandler::event_mask`](crate::arch::traits::strategy::EventHandler::event_mask)
/// to subscribe only to the broadcast channels it actually handles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EventMask(u32);

impl EventMask {
    /// Subscribe to no runtime event streams.
    pub const NONE: Self = Self(0);

    /// Generic alt-task lifecycle/control events.
    pub const ALT_EVENT: Self = Self(1 << 0);
    /// Generic websocket-task lifecycle/control events.
    pub const WS_EVENT: Self = Self(1 << 1);
    /// Order execution batches.
    pub const ORDER_EXECUTION: Self = Self(1 << 2);
    /// Instrument or portfolio target intents.
    pub const INST_INTENT: Self = Self(1 << 3);
    /// Model prediction tensors.
    pub const MODEL_PREDS: Self = Self(1 << 4);
    /// Periodic scheduler ticks.
    pub const SCHEDULE: Self = Self(1 << 5);
    /// Public trade batches.
    pub const TRADE: Self = Self(1 << 6);
    /// Public order book updates.
    pub const LOB: Self = Self(1 << 7);
    /// Public candle batches.
    pub const CANDLE: Self = Self(1 << 8);
    /// Private account order updates.
    pub const ACC_ORDER: Self = Self(1 << 9);
    /// Private balance and position updates.
    pub const ACC_BAL_POS: Self = Self(1 << 10);
    /// Private position-only updates.
    pub const ACC_POS: Self = Self(1 << 11);
    /// Public market-by-order order book updates.
    pub const LOB_MBO: Self = Self(1 << 12);
    /// Subscribe to every runtime event stream.
    pub const ALL: Self = Self((1 << 13) - 1);

    /// Returns [`EventMask::NONE`].
    pub const fn none() -> Self {
        Self::NONE
    }

    /// Returns [`EventMask::ALL`].
    pub const fn all() -> Self {
        Self::ALL
    }

    /// Returns true when all bits in `other` are present in this mask.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for EventMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for EventMask {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::EventMask;

    #[test]
    fn contains_combined_event_bits() {
        let mask =
            EventMask::TRADE | EventMask::ACC_BAL_POS | EventMask::MODEL_PREDS | EventMask::LOB_MBO;

        assert!(mask.contains(EventMask::TRADE));
        assert!(mask.contains(EventMask::ACC_BAL_POS));
        assert!(mask.contains(EventMask::MODEL_PREDS));
        assert!(mask.contains(EventMask::LOB_MBO));
        assert!(!mask.contains(EventMask::CANDLE));
    }

    #[test]
    fn supports_bit_or_assignment() {
        let mut mask = EventMask::none();
        mask |= EventMask::WS_EVENT;
        mask |= EventMask::TRADE;

        assert!(mask.contains(EventMask::WS_EVENT));
        assert!(mask.contains(EventMask::TRADE));
        assert!(!mask.contains(EventMask::ALT_EVENT));
    }
}
