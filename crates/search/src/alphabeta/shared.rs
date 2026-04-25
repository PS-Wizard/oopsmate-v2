use oopsmate_core::{Move, MoveKind, Piece, Position};
use oopsmate_memory::Bound;
use oopsmate_movegen::see_ge;

use crate::qsearch::NO_STATIC_EVAL;
use crate::tune::scale_eval;
use crate::types::is_mate_score;

pub(super) const LATE_BAD_CAPTURE_MIN_DEPTH: u8 = 3;
pub(super) const LATE_BAD_CAPTURE_MAX_DEPTH: u8 = 8;
pub(super) const LATE_BAD_CAPTURE_MIN_SEARCHED: usize = 4;
pub(super) const LATE_BAD_CAPTURE_MAX_GAIN: i32 = 330;

#[derive(Clone, Copy)]
pub(super) struct CaptureHistoryRecord {
    pub(super) moved: Piece,
    pub(super) to: usize,
    pub(super) captured: Piece,
}

impl CaptureHistoryRecord {
    pub(super) const EMPTY: Self = Self {
        moved: Piece::Pawn,
        to: 0,
        captured: Piece::Pawn,
    };
}

#[inline(always)]
#[must_use]
pub(super) fn pack_static_eval(score: i32) -> i16 {
    debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
    score as i16
}

#[inline(always)]
pub(super) fn should_update_correction(
    bound: Bound,
    in_check: bool,
    score: i32,
    raw_static_eval: i16,
) -> bool {
    bound == Bound::Exact
        && !in_check
        && raw_static_eval != NO_STATIC_EVAL
        && !is_mate_score(score)
        && !is_mate_score(i32::from(raw_static_eval))
}

#[inline(always)]
pub(super) fn should_prune_late_bad_capture(
    pos: &Position,
    mv: Move,
    tt_move: Move,
    maybe_check: bool,
    depth: u8,
    searched_moves: usize,
    alpha: i32,
    static_eval: i32,
    can_selectively_prune: bool,
) -> bool {
    let kind = mv.kind();
    if !can_selectively_prune
        || depth < LATE_BAD_CAPTURE_MIN_DEPTH
        || depth > LATE_BAD_CAPTURE_MAX_DEPTH
        || searched_moves < LATE_BAD_CAPTURE_MIN_SEARCHED
        || mv == tt_move
        || maybe_check
        || kind == MoveKind::EnPassant
        || kind.is_promotion()
        || !kind.is_capture()
        || is_mate_score(alpha)
        || see_ge(pos, mv, 0)
    {
        return false;
    }

    let captured = pos.piece_at(mv.to()).map_or(Piece::Pawn, |(piece, _)| piece);
    static_eval + scale_eval(LATE_BAD_CAPTURE_MAX_GAIN.min(piece_value(captured))) <= alpha
}

#[inline(always)]
const fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 320,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 0,
    }
}

pub(super) fn capture_history_record(pos: &Position, mv: Move) -> Option<CaptureHistoryRecord> {
    let kind = mv.kind();
    if kind.is_promotion() || !(kind.is_capture() || kind == MoveKind::EnPassant) {
        return None;
    }

    let moved = pos
        .piece_at(mv.from())
        .map_or(Piece::Pawn, |(piece, _)| piece);
    let captured = if kind == MoveKind::EnPassant {
        Piece::Pawn
    } else {
        pos.piece_at(mv.to())
            .map_or(Piece::Pawn, |(piece, _)| piece)
    };

    Some(CaptureHistoryRecord {
        moved,
        to: mv.to().index(),
        captured,
    })
}
