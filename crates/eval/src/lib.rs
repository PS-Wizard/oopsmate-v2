use std::io;

use nnuebie_v2::{NnueContext, NnueNetworks};
use oopsmate_core::{Move, Position};

pub trait Evaluator {
    #[inline(always)]
    fn reset(&mut self, _position: &Position) {}

    #[inline(always)]
    fn push_move(&mut self, _position: &Position, _mv: Move) {}

    #[inline(always)]
    fn pop_move(&mut self) {}

    fn evaluate(&mut self, position: &Position) -> i32;
}

#[derive(Debug)]
pub struct NnueEval {
    networks: NnueNetworks,
    context: NnueContext,
}

impl NnueEval {
    pub fn load_default() -> io::Result<Self> {
        Ok(Self {
            networks: NnueNetworks::load_default()?,
            context: NnueContext::new(),
        })
    }

    #[inline(always)]
    #[must_use]
    pub const fn networks(&self) -> &NnueNetworks {
        &self.networks
    }
}

impl Evaluator for NnueEval {
    #[inline(always)]
    fn reset(&mut self, position: &Position) {
        self.networks.reset_context(position, &mut self.context);
    }

    #[inline(always)]
    fn push_move(&mut self, position: &Position, mv: Move) {
        self.context.push_move(position, mv);
    }

    #[inline(always)]
    fn pop_move(&mut self) {
        self.context.pop();
    }

    #[inline(always)]
    fn evaluate(&mut self, position: &Position) -> i32 {
        self.networks.evaluate(position, &mut self.context).final_cp
    }
}
