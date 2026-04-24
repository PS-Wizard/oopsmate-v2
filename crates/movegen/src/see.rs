use oopsmate_core::{Color, Move, MoveKind, Piece, Position, Square};
use strikes::{bishop_attacks, knight_attacks, pawn_attacks, rook_attacks};

use crate::util::piece_bb;

const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0];
const MAX_EXCHANGES: usize = 32;

#[must_use]
pub fn see_ge(pos: &Position, mv: Move, threshold: i32) -> bool {
    let us = pos.side_to_move();
    let Some((attacker, color)) = pos.piece_at(mv.from()) else {
        return true;
    };
    debug_assert_eq!(color, us);

    if attacker == Piece::King || !is_supported_capture(mv) {
        return true;
    }

    let captured = captured_piece(pos, mv);
    let mut gain = [0i32; MAX_EXCHANGES];
    let mut depth = 0usize;
    let target = mv.to();
    let target_bit = target.bit();
    let mut occupied = pos.board().occupied() & !mv.from().bit();
    let mut side = us.flip();
    gain[0] = PIECE_VALUES[captured.index()];

    while let Some((from, piece)) = least_legal_attacker(pos, target, side, occupied) {
        depth += 1;
        if depth == MAX_EXCHANGES {
            break;
        }

        gain[depth] = PIECE_VALUES[piece.index()] - gain[depth - 1];
        occupied &= !from.bit();
        side = side.flip();

        if gain[depth].max(-gain[depth - 1]) < 0 {
            break;
        }

        debug_assert_ne!(occupied & target_bit, 0);
    }

    while depth > 0 {
        depth -= 1;
        gain[depth] = -gain[depth].max(-gain[depth + 1]);
    }

    gain[0] - threshold >= 0
}

#[inline(always)]
fn least_legal_attacker(
    pos: &Position,
    target: Square,
    side: Color,
    occupied: u64,
) -> Option<(Square, Piece)> {
    if let Some(square) = first_legal_attacker(
        pos,
        target,
        side,
        occupied,
        Piece::Pawn,
        pawn_attacks(side.flip().index(), target.index()) & piece_bb(pos, Piece::Pawn, side),
    ) {
        return Some((square, Piece::Pawn));
    }

    if let Some(square) = first_legal_attacker(
        pos,
        target,
        side,
        occupied,
        Piece::Knight,
        knight_attacks(target.index()) & piece_bb(pos, Piece::Knight, side),
    ) {
        return Some((square, Piece::Knight));
    }

    let bishops = piece_bb(pos, Piece::Bishop, side) | piece_bb(pos, Piece::Queen, side);
    let bishop_attackers = bishop_attacks(target.index(), occupied) & bishops;
    if let Some(square) = first_legal_attacker(
        pos,
        target,
        side,
        occupied,
        Piece::Bishop,
        bishop_attackers & piece_bb(pos, Piece::Bishop, side),
    ) {
        return Some((square, Piece::Bishop));
    }

    let rooks = piece_bb(pos, Piece::Rook, side) | piece_bb(pos, Piece::Queen, side);
    let rook_attackers = rook_attacks(target.index(), occupied) & rooks;
    if let Some(square) = first_legal_attacker(
        pos,
        target,
        side,
        occupied,
        Piece::Rook,
        rook_attackers & piece_bb(pos, Piece::Rook, side),
    ) {
        return Some((square, Piece::Rook));
    }

    first_legal_attacker(
        pos,
        target,
        side,
        occupied,
        Piece::Queen,
        (bishop_attackers | rook_attackers) & piece_bb(pos, Piece::Queen, side),
    )
    .map(|square| (square, Piece::Queen))
}

#[inline(always)]
fn first_legal_attacker(
    pos: &Position,
    target: Square,
    side: Color,
    occupied: u64,
    _piece: Piece,
    attackers: u64,
) -> Option<Square> {
    let mut attackers = attackers & occupied;
    while attackers != 0 {
        let bit = attackers & attackers.wrapping_neg();
        attackers ^= bit;
        let from = Square::from_raw(bit.trailing_zeros() as u8);
        if !exposes_king(pos, side, from, target, occupied) {
            return Some(from);
        }
    }

    None
}

#[inline(always)]
fn exposes_king(pos: &Position, side: Color, from: Square, target: Square, occupied: u64) -> bool {
    let king = pos.board().king_square(side);
    if king.is_none() {
        return false;
    }

    let occupied = (occupied & !from.bit()) | target.bit();
    let enemy = side.flip();
    let live_enemy = pos.board().color_bb(enemy) & occupied & !target.bit();
    let bishops = (piece_bb(pos, Piece::Bishop, enemy) | piece_bb(pos, Piece::Queen, enemy)) & live_enemy;
    if bishop_attacks(king.index(), occupied) & bishops != 0 {
        return true;
    }

    let rooks = (piece_bb(pos, Piece::Rook, enemy) | piece_bb(pos, Piece::Queen, enemy)) & live_enemy;
    rook_attacks(king.index(), occupied) & rooks != 0
}

#[inline(always)]
fn captured_piece(pos: &Position, mv: Move) -> Piece {
    if mv.kind() == MoveKind::EnPassant {
        Piece::Pawn
    } else {
        pos.piece_at(mv.to()).map_or(Piece::Pawn, |(piece, _)| piece)
    }
}

#[inline(always)]
const fn is_supported_capture(mv: Move) -> bool {
    let kind = (mv.0 >> 12) as u8;
    (kind & 0x4) != 0 && (kind & 0x8) == 0 && kind != MoveKind::EnPassant as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn winning_capture_passes() {
        let pos = Position::from_fen("4k3/8/8/3q4/4P3/8/8/4K3 w - - 0 1").unwrap();
        let mv = Move::new(sq("e4"), sq("d5"), MoveKind::Capture);

        assert!(see_ge(&pos, mv, 0));
    }

    #[test]
    fn losing_queen_capture_fails() {
        let pos = Position::from_fen("1r2k3/8/8/8/8/8/1p6/Q3K3 w - - 0 1").unwrap();
        let mv = Move::new(sq("a1"), sq("b2"), MoveKind::Capture);

        assert!(!see_ge(&pos, mv, 0));
    }

    #[test]
    fn pinned_recapturer_is_ignored() {
        let pos = Position::from_fen("4k3/4n3/8/3p4/2B5/8/8/4R1K1 w - - 0 1").unwrap();
        let mv = Move::new(sq("c4"), sq("d5"), MoveKind::Capture);

        assert!(see_ge(&pos, mv, 0));
    }
}
