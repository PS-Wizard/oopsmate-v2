use oopsmate_core::{Move, MoveKind, Piece, Position, Square};
use strikes::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks};

use crate::attacks::is_square_attacked;
#[must_use]
pub fn is_legal(pos: &Position, mv: Move) -> bool {
    if !is_pseudo_legal(pos, mv) {
        return false;
    }

    let us = pos.side_to_move();
    let mut next = pos.clone();
    next.make_move(mv);
    !is_square_attacked(&next, next.board().king_square(us), us.flip())
}

#[must_use]
pub fn is_pseudo_legal(pos: &Position, mv: Move) -> bool {
    let us = pos.side_to_move();
    let them = us.flip();
    let from = mv.from();
    let to = mv.to();
    if !from.is_valid() || !to.is_valid() || from == to {
        return false;
    }

    let Some((piece, color)) = pos.piece_at(from) else {
        return false;
    };
    if color != us {
        return false;
    }

    let target = pos.piece_at(to);
    if matches!(target, Some((_, c)) if c == us) {
        return false;
    }
    if matches!(target, Some((Piece::King, _))) {
        return false;
    }

    match piece {
        Piece::Pawn => is_pseudo_legal_pawn(pos, mv, us, target.is_some()),
        Piece::Knight => is_pseudo_legal_knight(mv, target.is_some()),
        Piece::Bishop => is_pseudo_legal_slider(pos, mv, target.is_some(), bishop_attacks),
        Piece::Rook => is_pseudo_legal_slider(pos, mv, target.is_some(), rook_attacks),
        Piece::Queen => is_pseudo_legal_slider(pos, mv, target.is_some(), |sq, occ| {
            bishop_attacks(sq, occ) | rook_attacks(sq, occ)
        }),
        Piece::King => is_pseudo_legal_king(pos, mv, us, them, target.is_some()),
    }
}

fn is_pseudo_legal_pawn(
    pos: &Position,
    mv: Move,
    us: oopsmate_core::Color,
    is_capture: bool,
) -> bool {
    let from = mv.from();
    let to = mv.to();

    match (us, mv.kind()) {
        (oopsmate_core::Color::White, MoveKind::Quiet) => {
            to.raw() == from.raw() + 8 && !is_capture && to.rank() < 7
        }
        (oopsmate_core::Color::Black, MoveKind::Quiet) => {
            from.raw() >= 8 && to.raw() + 8 == from.raw() && !is_capture && to.rank() > 0
        }
        (oopsmate_core::Color::White, MoveKind::DoublePush) => {
            from.rank() == 1
                && to.raw() == from.raw() + 16
                && pos.piece_at(Square::from_raw(from.raw() + 8)).is_none()
                && !is_capture
        }
        (oopsmate_core::Color::Black, MoveKind::DoublePush) => {
            from.rank() == 6
                && to.raw() + 16 == from.raw()
                && pos.piece_at(Square::from_raw(from.raw() - 8)).is_none()
                && !is_capture
        }
        (_, MoveKind::Capture) => {
            pawn_attacks(us.index(), from.index()) & to.bit() != 0 && is_capture
        }
        (_, MoveKind::EnPassant) => {
            pos.ep_square() == to && pawn_attacks(us.index(), from.index()) & to.bit() != 0
        }
        (
            oopsmate_core::Color::White,
            MoveKind::PromotionKnight
            | MoveKind::PromotionBishop
            | MoveKind::PromotionRook
            | MoveKind::PromotionQueen,
        ) => from.rank() == 6 && to.raw() == from.raw() + 8 && !is_capture,
        (
            oopsmate_core::Color::Black,
            MoveKind::PromotionKnight
            | MoveKind::PromotionBishop
            | MoveKind::PromotionRook
            | MoveKind::PromotionQueen,
        ) => from.rank() == 1 && to.raw() + 8 == from.raw() && !is_capture,
        (
            oopsmate_core::Color::White,
            MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen,
        ) => {
            from.rank() == 6 && pawn_attacks(us.index(), from.index()) & to.bit() != 0 && is_capture
        }
        (
            oopsmate_core::Color::Black,
            MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen,
        ) => {
            from.rank() == 1 && pawn_attacks(us.index(), from.index()) & to.bit() != 0 && is_capture
        }
        _ => false,
    }
}

fn is_pseudo_legal_knight(mv: Move, is_capture: bool) -> bool {
    if !matches!(mv.kind(), MoveKind::Quiet | MoveKind::Capture) {
        return false;
    }

    let attacks = knight_attacks(mv.from().index());
    attacks & mv.to().bit() != 0 && is_capture == mv.kind().is_capture()
}

fn is_pseudo_legal_slider(
    pos: &Position,
    mv: Move,
    is_capture: bool,
    attacks_from: fn(usize, u64) -> u64,
) -> bool {
    if mv.kind().is_promotion()
        || matches!(
            mv.kind(),
            MoveKind::DoublePush | MoveKind::Castle | MoveKind::EnPassant
        )
    {
        return false;
    }

    let attacks = attacks_from(mv.from().index(), pos.board().occupied());
    attacks & mv.to().bit() != 0 && is_capture == mv.kind().is_capture()
}

fn is_pseudo_legal_king(
    pos: &Position,
    mv: Move,
    us: oopsmate_core::Color,
    _them: oopsmate_core::Color,
    is_capture: bool,
) -> bool {
    match mv.kind() {
        MoveKind::Quiet | MoveKind::Capture => {
            king_attacks(mv.from().index()) & mv.to().bit() != 0
                && is_capture == mv.kind().is_capture()
        }
        MoveKind::Castle => match (us, mv.from().raw(), mv.to().raw()) {
            (oopsmate_core::Color::White, 4, 6) => {
                pos.castling().can_castle_kingside(us)
                    && pos.piece_at(Square::from_raw(5)).is_none()
                    && pos.piece_at(Square::from_raw(6)).is_none()
            }
            (oopsmate_core::Color::White, 4, 2) => {
                pos.castling().can_castle_queenside(us)
                    && pos.piece_at(Square::from_raw(1)).is_none()
                    && pos.piece_at(Square::from_raw(2)).is_none()
                    && pos.piece_at(Square::from_raw(3)).is_none()
            }
            (oopsmate_core::Color::Black, 60, 62) => {
                pos.castling().can_castle_kingside(us)
                    && pos.piece_at(Square::from_raw(61)).is_none()
                    && pos.piece_at(Square::from_raw(62)).is_none()
            }
            (oopsmate_core::Color::Black, 60, 58) => {
                pos.castling().can_castle_queenside(us)
                    && pos.piece_at(Square::from_raw(57)).is_none()
                    && pos.piece_at(Square::from_raw(58)).is_none()
                    && pos.piece_at(Square::from_raw(59)).is_none()
            }
            _ => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn legal_filter_rejects_moving_into_check() {
        let pos = Position::from_fen("4k3/8/8/8/8/3n4/8/4K3 w - - 0 1").unwrap();
        let mv = Move::new(sq("e1"), sq("f2"), MoveKind::Quiet);

        assert!(is_pseudo_legal(&pos, mv));
        assert!(!is_legal(&pos, mv));
    }

    #[test]
    fn legal_filter_accepts_castling_when_clear() {
        let pos = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let mv = Move::new(sq("e1"), sq("g1"), MoveKind::Castle);

        assert!(is_pseudo_legal(&pos, mv));
        assert!(is_legal(&pos, mv));
    }
}
