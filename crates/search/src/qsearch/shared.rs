use oopsmate_core::{Move, MoveKind, Piece, Position};
use oopsmate_movegen::see_ge;

use crate::tune::{scale_eval, QSEARCH_CAPTURE_BASE, QSEARCH_DELTA_MARGIN, QSEARCH_PROMOTION_BASE};
use crate::types::is_mate_score;

pub(crate) const NO_STATIC_EVAL: i16 = i16::MIN;
const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0];

#[inline(always)]
pub(super) fn delta_prune_move(pos: &Position, mv: Move, static_eval: i32, alpha: i32) -> bool {
    if is_mate_score(alpha) || mv.is_promotion() {
        return false;
    }

    let captured = captured_piece(pos, mv);
    static_eval + delta_piece_value(captured) + scale_eval(QSEARCH_DELTA_MARGIN) <= alpha
}

#[inline(always)]
pub(super) fn see_prune_move(pos: &Position, mv: Move) -> bool {
    mv.is_capture() && !mv.is_promotion() && mv.kind() != MoveKind::EnPassant && !see_ge(pos, mv, 0)
}

#[inline(always)]
pub(super) fn score_qmove(pos: &Position, mv: Move) -> i16 {
    let kind = (mv.0 >> 12) as u8;
    let mut score = 0;

    if (kind & 0x8) != 0 {
        score += QSEARCH_PROMOTION_BASE + PIECE_VALUES[((kind & 0x3) as usize) + 1];
    }

    if (kind & 0x4) != 0 || kind == MoveKind::EnPassant as u8 {
        let attacker = pos
            .piece_at(mv.from())
            .map_or(Piece::Pawn, |(piece, _)| piece);
        let captured = captured_piece(pos, mv);

        score += QSEARCH_CAPTURE_BASE + PIECE_VALUES[captured.index()] * 16
            - PIECE_VALUES[attacker.index()];
    }

    debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
    score as i16
}

#[inline(always)]
const fn delta_piece_value(piece: Piece) -> i32 {
    scale_eval(PIECE_VALUES[piece.index()])
}

#[inline(always)]
pub(super) fn captured_piece(pos: &Position, mv: Move) -> Piece {
    if ((mv.0 >> 12) as u8) == MoveKind::EnPassant as u8 {
        Piece::Pawn
    } else {
        pos.piece_at(mv.to())
            .map_or(Piece::Pawn, |(piece, _)| piece)
    }
}

#[inline(always)]
pub(super) const fn is_tactical_move(mv: Move) -> bool {
    let kind = (mv.0 >> 12) as u8;
    (kind & 0x4) != 0 || (kind & 0x8) != 0 || kind == MoveKind::EnPassant as u8
}

#[inline(always)]
pub(super) const fn is_valid_encoded_move(mv: Move) -> bool {
    matches!((mv.0 >> 12) as u8, 0..=4 | 8..=15)
}

#[inline(always)]
#[must_use]
pub(super) fn pack_static_eval(score: i32) -> i16 {
    debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
    score as i16
}

#[cfg(test)]
mod tests {
    use super::*;
    use oopsmate_core::Square;

    fn square(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn delta_pruning_never_skips_promotions() {
        let pos = Position::from_fen("4k3/3P4/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let promotion = Move::new(square("d7"), square("d8"), MoveKind::PromotionQueen);

        assert!(!delta_prune_move(
            &pos,
            promotion,
            scale_eval(-1000),
            scale_eval(500),
        ));
    }

    #[test]
    fn delta_pruning_skips_hopeless_small_capture() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/p7/R3K3 w - - 0 1").unwrap();
        let capture = Move::new(square("a1"), square("a2"), MoveKind::Capture);

        assert!(delta_prune_move(&pos, capture, scale_eval(-600), 0));
    }
}
