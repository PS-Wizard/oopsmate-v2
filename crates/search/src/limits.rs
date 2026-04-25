use std::time::Duration;

use oopsmate_core::Color;

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchLimits {
    pub depth: Option<u8>,
    pub movetime_ms: Option<u64>,
    pub clock: Option<ClockLimits>,
}

impl SearchLimits {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            depth: None,
            movetime_ms: None,
            clock: None,
        }
    }

    #[must_use]
    pub const fn depth(depth: u8) -> Self {
        Self {
            depth: Some(depth),
            movetime_ms: None,
            clock: None,
        }
    }

    #[must_use]
    pub const fn movetime(movetime_ms: u64) -> Self {
        Self {
            depth: None,
            movetime_ms: Some(movetime_ms),
            clock: None,
        }
    }

    #[must_use]
    pub const fn with_clock(self, clock: ClockLimits) -> Self {
        Self {
            depth: self.depth,
            movetime_ms: self.movetime_ms,
            clock: Some(clock),
        }
    }

    #[must_use]
    pub(crate) fn deadlines(self, side_to_move: Color) -> (Option<Duration>, Option<Duration>) {
        if let Some(movetime_ms) = self.movetime_ms {
            let safety_ms = movetime_ms.saturating_div(40).max(10).min(movetime_ms);
            let soft_ms = movetime_ms.saturating_sub(safety_ms);
            return (
                Some(Duration::from_millis(soft_ms)),
                Some(Duration::from_millis(movetime_ms)),
            );
        }

        let Some(clock) = self.clock else {
            return (None, None);
        };

        let (time_left_ms, increment_ms) = clock.for_side(side_to_move);
        if time_left_ms == 0 {
            return (Some(Duration::ZERO), Some(Duration::ZERO));
        }

        let reserve_ms = time_left_ms.saturating_div(40).max(10).min(250);
        let usable_ms = time_left_ms.saturating_sub(reserve_ms);

        let mut soft_ms = if let Some(movestogo) = clock.movestogo.filter(|&value| value > 0) {
            usable_ms / movestogo + increment_ms / 2
        } else {
            usable_ms / 12 + increment_ms * 3 / 4
        };

        if usable_ms > 0 {
            soft_ms = soft_ms.clamp(1, usable_ms);
        }

        let hard_ms = (soft_ms + soft_ms / 2).min(time_left_ms.saturating_sub(1).max(soft_ms));

        (
            Some(Duration::from_millis(soft_ms)),
            Some(Duration::from_millis(hard_ms)),
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ClockLimits {
    pub white_time_ms: u64,
    pub black_time_ms: u64,
    pub white_increment_ms: u64,
    pub black_increment_ms: u64,
    pub movestogo: Option<u64>,
}

impl ClockLimits {
    #[inline(always)]
    #[must_use]
    fn for_side(self, side_to_move: Color) -> (u64, u64) {
        match side_to_move {
            Color::White => (self.white_time_ms, self.white_increment_ms),
            Color::Black => (self.black_time_ms, self.black_increment_ms),
        }
    }
}
