use oopsmate_core::{Color, Move};

const HISTORY_SIZE: usize = 64 * 64;

#[derive(Debug)]
pub struct HistoryTable {
    quiet: [[i32; HISTORY_SIZE]; 2],
}

impl HistoryTable {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            quiet: [[0; HISTORY_SIZE]; 2],
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.quiet = [[0; HISTORY_SIZE]; 2];
    }

    #[inline(always)]
    #[must_use]
    pub fn score(&self, side: Color, mv: Move) -> i16 {
        clamp_i16(self.quiet[side.index()][index(mv)])
    }

    #[inline(always)]
    pub fn reward_quiet_cutoff(&mut self, side: Color, mv: Move, depth: u8) {
        let slot = &mut self.quiet[side.index()][index(mv)];
        let bonus = i32::from(depth) * i32::from(depth);
        *slot = slot.saturating_add(bonus);
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

#[inline(always)]
const fn index(mv: Move) -> usize {
    (mv.from().index() << 6) | mv.to().index()
}

#[inline(always)]
const fn clamp_i16(score: i32) -> i16 {
    if score < i16::MIN as i32 {
        i16::MIN
    } else if score > i16::MAX as i32 {
        i16::MAX
    } else {
        score as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oopsmate_core::{MoveKind, Square};

    #[test]
    fn quiet_cutoff_reward_increases_score() {
        let mv = Move::new(
            Square::from_algebraic("d2").unwrap(),
            Square::from_algebraic("d4").unwrap(),
            MoveKind::DoublePush,
        );
        let mut history = HistoryTable::new();

        assert_eq!(history.score(Color::White, mv), 0);
        history.reward_quiet_cutoff(Color::White, mv, 5);
        assert_eq!(history.score(Color::White, mv), 25);
    }
}
