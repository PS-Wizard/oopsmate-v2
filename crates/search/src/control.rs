use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use oopsmate_core::Color;

use crate::limits::SearchLimits;

pub(crate) struct SearchContext<'a> {
    start: Instant,
    stop: &'a AtomicBool,
    soft_deadline: Option<Instant>,
    hard_deadline: Option<Instant>,
    nodes: u64,
}

pub(crate) struct SearchInterrupted;

impl<'a> SearchContext<'a> {
    #[must_use]
    pub(crate) fn new(stop: &'a AtomicBool, limits: SearchLimits, side_to_move: Color) -> Self {
        let start = Instant::now();
        let (soft_limit, hard_limit) = limits.deadlines(side_to_move);

        Self {
            start,
            stop,
            soft_deadline: soft_limit.map(|limit| start + limit),
            hard_deadline: hard_limit.map(|limit| start + limit),
            nodes: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn enter_node(&mut self) -> Result<(), SearchInterrupted> {
        self.nodes += 1;

        if (self.nodes & 1023) == 0 && self.should_stop_now() {
            return Err(SearchInterrupted);
        }

        Ok(())
    }

    #[inline(always)]
    pub(crate) fn should_stop_now(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
            || self
                .hard_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
    }

    #[inline(always)]
    #[must_use]
    pub(crate) fn reached_soft_deadline(&self) -> bool {
        self.soft_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }

    #[inline(always)]
    #[must_use]
    pub(crate) const fn nodes(&self) -> u64 {
        self.nodes
    }

    #[must_use]
    pub(crate) fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}
