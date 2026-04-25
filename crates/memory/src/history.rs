use oopsmate_core::{Color, Move, Piece};

const HISTORY_SIZE: usize = 64 * 64;
const HISTORY_LIMIT: i32 = 16_384;
const CAPTURE_HISTORY_SIZE: usize = 2 * 6 * 64 * 6;

#[derive(Debug)]
pub struct HistoryTable {
    quiet: [[i32; HISTORY_SIZE]; 2],
    capture: [i32; CAPTURE_HISTORY_SIZE],
}

impl HistoryTable {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            quiet: [[0; HISTORY_SIZE]; 2],
            capture: [0; CAPTURE_HISTORY_SIZE],
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.quiet = [[0; HISTORY_SIZE]; 2];
        self.capture = [0; CAPTURE_HISTORY_SIZE];
    }

    #[inline(always)]
    #[must_use]
    pub fn score(&self, side: Color, mv: Move) -> i16 {
        clamp_i16(self.quiet[side.index()][index(mv)])
    }

    #[inline(always)]
    pub fn reward_quiet_cutoff(&mut self, side: Color, mv: Move, depth: u8) {
        gravity_update(
            &mut self.quiet[side.index()][index(mv)],
            history_bonus(depth),
        );
    }

    #[inline(always)]
    pub fn penalize_quiet_fail(&mut self, side: Color, mv: Move, depth: u8) {
        gravity_update(
            &mut self.quiet[side.index()][index(mv)],
            -history_bonus(depth) / 2,
        );
    }

    #[inline(always)]
    #[must_use]
    pub fn capture_score(&self, side: Color, moved: Piece, to: usize, captured: Piece) -> i16 {
        clamp_i16(self.capture[capture_index(side, moved, to, captured)])
    }

    #[inline(always)]
    pub fn reward_capture_cutoff(
        &mut self,
        side: Color,
        moved: Piece,
        to: usize,
        captured: Piece,
        depth: u8,
    ) {
        gravity_update(
            &mut self.capture[capture_index(side, moved, to, captured)],
            history_bonus(depth),
        );
    }

    #[inline(always)]
    pub fn penalize_capture_fail(
        &mut self,
        side: Color,
        moved: Piece,
        to: usize,
        captured: Piece,
        depth: u8,
    ) {
        gravity_update(
            &mut self.capture[capture_index(side, moved, to, captured)],
            -history_bonus(depth) / 2,
        );
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

#[inline(always)]
fn gravity_update(slot: &mut i32, delta: i32) {
    let bonus = delta.clamp(-HISTORY_LIMIT, HISTORY_LIMIT);
    *slot += bonus - *slot * bonus.abs() / HISTORY_LIMIT;
    *slot = (*slot).clamp(-HISTORY_LIMIT, HISTORY_LIMIT);
}

#[inline(always)]
const fn history_bonus(depth: u8) -> i32 {
    let depth = depth as i32;
    depth * depth
}

#[inline(always)]
const fn index(mv: Move) -> usize {
    (mv.from().index() << 6) | mv.to().index()
}

#[inline(always)]
const fn capture_index(side: Color, moved: Piece, to: usize, captured: Piece) -> usize {
    (((side.index() * 6 + moved.index()) * 64 + to) * 6) + captured.index()
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

    #[test]
    fn quiet_fail_penalty_decreases_score() {
        let mv = Move::new(
            Square::from_algebraic("d2").unwrap(),
            Square::from_algebraic("d4").unwrap(),
            MoveKind::DoublePush,
        );
        let mut history = HistoryTable::new();

        history.reward_quiet_cutoff(Color::White, mv, 6);
        history.penalize_quiet_fail(Color::White, mv, 4);

        assert_eq!(history.score(Color::White, mv), 28);
    }

    #[test]
    fn gravity_update_soft_saturates_repeated_rewards() {
        let mv = Move::new(
            Square::from_algebraic("g1").unwrap(),
            Square::from_algebraic("f3").unwrap(),
            MoveKind::Quiet,
        );
        let mut history = HistoryTable::new();

        for _ in 0..400 {
            history.reward_quiet_cutoff(Color::White, mv, 16);
        }

        assert!(history.score(Color::White, mv) <= HISTORY_LIMIT as i16);
        assert!(history.score(Color::White, mv) > 16_000);
    }

    #[test]
    fn gravity_update_pulls_overconfident_scores_down() {
        let mv = Move::new(
            Square::from_algebraic("g1").unwrap(),
            Square::from_algebraic("f3").unwrap(),
            MoveKind::Quiet,
        );
        let mut history = HistoryTable::new();

        for _ in 0..400 {
            history.reward_quiet_cutoff(Color::White, mv, 16);
        }
        let before = history.score(Color::White, mv);
        history.penalize_quiet_fail(Color::White, mv, 16);

        assert!(history.score(Color::White, mv) < before - 250);
    }

    #[test]
    fn capture_history_updates_by_piece_to_captured() {
        let mut history = HistoryTable::new();

        assert_eq!(
            history.capture_score(Color::White, Piece::Knight, 42, Piece::Pawn),
            0
        );
        history.reward_capture_cutoff(Color::White, Piece::Knight, 42, Piece::Pawn, 5);

        assert_eq!(
            history.capture_score(Color::White, Piece::Knight, 42, Piece::Pawn),
            25
        );
        assert_eq!(
            history.capture_score(Color::Black, Piece::Knight, 42, Piece::Pawn),
            0
        );
    }
}
