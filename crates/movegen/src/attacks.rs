use oopsmate_core::{Color, Piece, Position, Square};
use strikes::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks};

use crate::util::piece_bb;

#[inline(always)]
#[must_use]
pub fn is_square_attacked(pos: &Position, square: Square, by: Color) -> bool {
    is_square_attacked_with_occ(pos, square, by, pos.board().occupied())
}

#[inline(always)]
#[must_use]
pub fn is_square_attacked_with_occ(
    pos: &Position,
    square: Square,
    by: Color,
    occupied: u64,
) -> bool {
    // The explicit occupancy override is used by king move generation and EP
    // validation, where legality depends on a position that does not yet exist
    // as a full Position value.
    let sq = square.index();

    if knight_attacks(sq) & piece_bb(pos, Piece::Knight, by) != 0 {
        return true;
    }

    if king_attacks(sq) & piece_bb(pos, Piece::King, by) != 0 {
        return true;
    }

    if pawn_attacks(by.flip().index(), sq) & piece_bb(pos, Piece::Pawn, by) != 0 {
        return true;
    }

    let bishops = piece_bb(pos, Piece::Bishop, by) | piece_bb(pos, Piece::Queen, by);
    if bishop_attacks(sq, occupied) & bishops != 0 {
        return true;
    }

    let rooks = piece_bb(pos, Piece::Rook, by) | piece_bb(pos, Piece::Queen, by);
    rook_attacks(sq, occupied) & rooks != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn attacked_square_queries_match_expected_attackers() {
        let pos = Position::from_fen("4k3/8/3n4/8/4P3/8/8/4K3 b - - 0 1").unwrap();

        assert!(is_square_attacked(&pos, sq("e4"), Color::Black));
        assert!(!is_square_attacked(&pos, sq("e4"), Color::White));
    }
}
