use std::mem::MaybeUninit;

use oopsmate_core::{Color, Move, MoveKind, Piece, Position};
use oopsmate_memory::HistoryTable;
use oopsmate_movegen::{
    generate_captures_promotions_with_analysis, generate_evasions_with_analysis,
    generate_quiets_with_analysis, see_ge, Analysis, MoveList, MAX_MOVES,
};

use crate::tune::{
    MOVE_PICKER_BAD_CAPTURE_BASE, MOVE_PICKER_CAPTURE_BASE, MOVE_PICKER_PROMOTION_BASE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TtMode {
    ValidateInStage,
    BlindTrust,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Tt,
    Captures,
    Quiets,
    Evasions,
    Done,
}

pub(crate) struct MovePicker {
    phase: Phase,
    side: Color,
    tt_move: Move,
    tt_mode: TtMode,
    tt_yielded: bool,
    in_check: bool,
    stage_loaded: bool,
    moves: MoveList,
    scores: MaybeUninit<[i16; MAX_MOVES]>,
    next: usize,
}

impl MovePicker {
    #[must_use]
    pub(crate) fn new(pos: &Position, analysis: &Analysis, tt_move: Move, tt_mode: TtMode) -> Self {
        let tt_move = match tt_mode {
            TtMode::ValidateInStage | TtMode::BlindTrust
                if tt_move != Move::NULL && !has_valid_kind(tt_move) =>
            {
                Move::NULL
            }
            TtMode::ValidateInStage | TtMode::BlindTrust => tt_move,
        };

        Self {
            phase: Phase::Tt,
            side: pos.side_to_move(),
            tt_move,
            tt_mode,
            tt_yielded: false,
            in_check: analysis.in_check(),
            stage_loaded: false,
            moves: MoveList::new(),
            scores: MaybeUninit::uninit(),
            next: 0,
        }
    }

    pub(crate) fn next_move(
        &mut self,
        pos: &Position,
        analysis: &Analysis,
        history: &HistoryTable,
    ) -> Option<Move> {
        loop {
            match self.phase {
                Phase::Tt => {
                    self.phase = self.first_generated_phase();
                    if let Some(mv) = self.try_tt_move(pos, analysis, history) {
                        self.tt_yielded = true;
                        return Some(mv);
                    }
                }
                Phase::Captures | Phase::Quiets | Phase::Evasions => {
                    if !self.stage_loaded {
                        self.load_stage(pos, analysis, history);
                    }

                    if let Some(mv) = self.pick_best() {
                        return Some(mv);
                    }

                    self.advance_phase();
                }
                Phase::Done => return None,
            }
        }
    }

    #[inline(always)]
    fn first_generated_phase(&self) -> Phase {
        if self.in_check {
            Phase::Evasions
        } else {
            Phase::Captures
        }
    }

    fn try_tt_move(
        &mut self,
        pos: &Position,
        analysis: &Analysis,
        history: &HistoryTable,
    ) -> Option<Move> {
        if self.tt_move == Move::NULL {
            return None;
        }

        match self.tt_mode {
            TtMode::BlindTrust => Some(self.tt_move),
            TtMode::ValidateInStage => {
                let validation_phase = if self.in_check {
                    Phase::Evasions
                } else if is_tactical_move(self.tt_move) {
                    Phase::Captures
                } else {
                    Phase::Quiets
                };

                if validation_phase == self.phase {
                    if !self.stage_loaded {
                        self.load_stage(pos, analysis, history);
                    }
                    self.moves.contains(self.tt_move).then_some(self.tt_move)
                } else {
                    let mut generated = MoveList::new();
                    generate_quiets_with_analysis(pos, analysis, &mut generated);
                    generated.contains(self.tt_move).then_some(self.tt_move)
                }
            }
        }
    }

    fn load_stage(&mut self, pos: &Position, analysis: &Analysis, history: &HistoryTable) {
        self.moves.clear();
        match self.phase {
            Phase::Captures => {
                generate_captures_promotions_with_analysis(pos, analysis, &mut self.moves)
            }
            Phase::Quiets => generate_quiets_with_analysis(pos, analysis, &mut self.moves),
            Phase::Evasions => generate_evasions_with_analysis(pos, analysis, &mut self.moves),
            Phase::Tt | Phase::Done => unreachable!(),
        }

        self.next = 0;
        self.stage_loaded = true;

        for index in 0..self.moves.len() {
            let mv = self.moves.as_slice()[index];
            let score = match self.phase {
                Phase::Quiets => history.score(self.side, mv),
                Phase::Captures | Phase::Evasions => score_move(pos, mv, self.side, history),
                Phase::Tt | Phase::Done => unreachable!(),
            };
            self.write_score(index, score);
        }
    }

    fn pick_best(&mut self) -> Option<Move> {
        while self.next < self.moves.len() {
            let mut best = self.next;
            for index in (self.next + 1)..self.moves.len() {
                if self.score(index) > self.score(best) {
                    best = index;
                }
            }

            self.moves.swap(self.next, best);
            self.swap_scores(self.next, best);

            let mv = self.moves.as_slice()[self.next];
            self.next += 1;

            if self.tt_yielded && mv == self.tt_move {
                continue;
            }

            return Some(mv);
        }

        None
    }

    fn advance_phase(&mut self) {
        self.stage_loaded = false;
        self.phase = match self.phase {
            Phase::Captures => Phase::Quiets,
            Phase::Quiets | Phase::Evasions => Phase::Done,
            Phase::Tt | Phase::Done => Phase::Done,
        };
    }

    #[inline(always)]
    fn write_score(&mut self, index: usize, score: i16) {
        debug_assert!(index < MAX_MOVES);
        unsafe {
            (self.scores.as_mut_ptr() as *mut i16)
                .add(index)
                .write(score);
        }
    }

    #[inline(always)]
    fn score(&self, index: usize) -> i16 {
        debug_assert!(index < self.moves.len());
        unsafe { *((self.scores.as_ptr() as *const i16).add(index)) }
    }

    #[inline(always)]
    fn swap_scores(&mut self, a: usize, b: usize) {
        debug_assert!(a < self.moves.len());
        debug_assert!(b < self.moves.len());
        if a == b {
            return;
        }

        unsafe {
            std::ptr::swap(
                (self.scores.as_mut_ptr() as *mut i16).add(a),
                (self.scores.as_mut_ptr() as *mut i16).add(b),
            );
        }
    }
}

const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0];
const CAPTURE_VICTIM_SCALE: i32 = 8;

#[inline(always)]
fn score_move(pos: &Position, mv: Move, side: Color, history: &HistoryTable) -> i16 {
    let kind = mv.kind();
    let mut score = 0;

    if kind.is_promotion() {
        let promoted = kind.promotion_piece().expect("promotion piece");
        let captured = if kind.is_capture() {
            PIECE_VALUES[captured_piece(pos, mv).index()]
        } else {
            0
        };
        score += MOVE_PICKER_PROMOTION_BASE + PIECE_VALUES[promoted.index()] + captured;
        debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
        return score as i16;
    }

    if kind.is_capture() || kind == MoveKind::EnPassant {
        let attacker = pos
            .piece_at(mv.from())
            .map_or(Piece::Pawn, |(piece, _)| piece);
        let captured = captured_piece(pos, mv);
        let base = if is_good_capture(pos, mv) {
            MOVE_PICKER_CAPTURE_BASE
        } else {
            MOVE_PICKER_BAD_CAPTURE_BASE
        };

        score += base + PIECE_VALUES[captured.index()] * CAPTURE_VICTIM_SCALE
            - PIECE_VALUES[attacker.index()]
            + i32::from(history.capture_score(side, attacker, mv.to().index(), captured));
    }

    debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
    score as i16
}

#[inline(always)]
fn captured_piece(pos: &Position, mv: Move) -> Piece {
    if mv.kind() == MoveKind::EnPassant {
        Piece::Pawn
    } else {
        pos.piece_at(mv.to())
            .map_or(Piece::Pawn, |(piece, _)| piece)
    }
}

#[inline(always)]
fn is_good_capture(pos: &Position, mv: Move) -> bool {
    mv.kind() == MoveKind::EnPassant || see_ge(pos, mv, 0)
}

#[inline(always)]
const fn is_tactical_move(mv: Move) -> bool {
    let kind = (mv.0 >> 12) as u8;
    (kind & 0x4) != 0 || (kind & 0x8) != 0 || kind == MoveKind::EnPassant as u8
}

#[inline(always)]
const fn has_valid_kind(mv: Move) -> bool {
    matches!((mv.0 >> 12) as u8, 0..=4 | 8..=15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oopsmate_core::{MoveKind, Square};
    use oopsmate_movegen::analyze;

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn validate_in_stage_yields_tt_quiet_before_captures() {
        let pos = Position::from_fen("4k3/8/8/2n5/3P4/8/8/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);
        let tt_move = Move::new(sq("d4"), sq("d5"), MoveKind::Quiet);
        let history = HistoryTable::new();
        let mut picker = MovePicker::new(&pos, &analysis, tt_move, TtMode::ValidateInStage);

        assert_eq!(picker.next_move(&pos, &analysis, &history), Some(tt_move));
    }

    #[test]
    fn validate_in_stage_rejects_bogus_tt_move() {
        let pos = Position::from_fen("4k3/8/8/2n5/3P4/8/8/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);
        let bogus = Move::new(sq("a1"), sq("a2"), MoveKind::Quiet);
        let history = HistoryTable::new();
        let mut picker = MovePicker::new(&pos, &analysis, bogus, TtMode::ValidateInStage);

        assert_ne!(picker.next_move(&pos, &analysis, &history), Some(bogus));
    }

    #[test]
    fn blind_trust_mode_still_rejects_invalid_move_kind() {
        let pos = Position::from_fen("4k3/8/8/2n5/3P4/8/8/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);
        let bogus = Move((sq("a1").raw() as u16) | ((sq("a2").raw() as u16) << 6) | (5 << 12));
        let history = HistoryTable::new();
        let mut picker = MovePicker::new(&pos, &analysis, bogus, TtMode::BlindTrust);

        assert_ne!(picker.next_move(&pos, &analysis, &history), Some(bogus));
    }

    #[test]
    fn yielded_tt_move_is_not_repeated_from_stage() {
        let pos = Position::from_fen("4k3/8/8/2n5/3P4/8/8/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);
        let tt_move = Move::new(sq("d4"), sq("c5"), MoveKind::Capture);
        let history = HistoryTable::new();
        let mut picker = MovePicker::new(&pos, &analysis, tt_move, TtMode::ValidateInStage);
        let mut seen = 0;

        while let Some(mv) = picker.next_move(&pos, &analysis, &history) {
            if mv == tt_move {
                seen += 1;
            }
        }

        assert_eq!(seen, 1);
    }

    #[test]
    fn quiet_stage_uses_history_scores() {
        let pos = Position::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);
        let quiet = Move::new(sq("d4"), sq("f5"), MoveKind::Quiet);
        let mut history = HistoryTable::new();
        history.reward_quiet_cutoff(Color::White, quiet, 8);
        let mut picker = MovePicker::new(&pos, &analysis, Move::NULL, TtMode::BlindTrust);

        assert_eq!(picker.next_move(&pos, &analysis, &history), Some(quiet));
    }

    #[test]
    fn winning_capture_stays_before_quiets() {
        let pos = Position::from_fen("4k3/8/8/3q4/4P3/8/8/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);
        let capture = Move::new(sq("e4"), sq("d5"), MoveKind::Capture);
        let quiet = Move::new(sq("e1"), sq("d1"), MoveKind::Quiet);
        let mut history = HistoryTable::new();
        history.reward_quiet_cutoff(Color::White, quiet, 8);
        let mut picker = MovePicker::new(&pos, &analysis, Move::NULL, TtMode::BlindTrust);

        assert_eq!(picker.next_move(&pos, &analysis, &history), Some(capture));
    }
}
