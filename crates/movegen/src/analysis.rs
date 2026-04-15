use oopsmate_core::{Color, Piece, Position, Square};
use strikes::{
    bishop_attacks, knight_attacks, line_between, line_through, pawn_attacks, rook_attacks,
};

use crate::util::{color_bb, piece_bb, pop_lsb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Analysis {
    pub us: Color,
    pub them: Color,
    pub king_sq: Square,
    pub us_occ: u64,
    pub them_occ: u64,
    pub occ: u64,
    pub pinned: u64,
    pub checkers: u64,
    pub check_mask: u64,
}

impl Analysis {
    #[inline(always)]
    #[must_use]
    pub const fn in_check(self) -> bool {
        self.checkers != 0
    }

    #[inline(always)]
    #[must_use]
    pub const fn double_check(self) -> bool {
        self.checkers.count_ones() > 1
    }

    #[inline(always)]
    #[must_use]
    pub fn is_pinned(self, square: Square) -> bool {
        self.pinned & square.bit() != 0
    }

    #[inline(always)]
    #[must_use]
    pub fn pin_ray(self, square: Square) -> u64 {
        line_through(self.king_sq.index(), square.index())
    }
}

#[inline(always)]
#[must_use]
pub fn analyze(pos: &Position) -> Analysis {
    let us = pos.side_to_move();
    let them = us.flip();
    let king_sq = pos.board().king_square(us);
    let us_occ = color_bb(pos, us);
    let them_occ = color_bb(pos, them);
    let occ = us_occ | them_occ;

    let enemy_bishops_queens =
        piece_bb(pos, Piece::Bishop, them) | piece_bb(pos, Piece::Queen, them);
    let enemy_rooks_queens = piece_bb(pos, Piece::Rook, them) | piece_bb(pos, Piece::Queen, them);

    let mut pinned = 0u64;
    let mut checkers = 0u64;

    let mut potential = (bishop_attacks(king_sq.index(), 0) & enemy_bishops_queens)
        | (rook_attacks(king_sq.index(), 0) & enemy_rooks_queens);

    while potential != 0 {
        let attacker_sq = pop_lsb(&mut potential);
        let between = line_between(king_sq.index(), attacker_sq.index());
        let blockers = between & occ;

        if blockers == 0 {
            checkers |= attacker_sq.bit();
        } else if blockers.count_ones() == 1 && blockers & us_occ != 0 {
            pinned |= blockers;
        }
    }

    checkers |= knight_attacks(king_sq.index()) & piece_bb(pos, Piece::Knight, them);
    checkers |= pawn_attacks(us.index(), king_sq.index()) & piece_bb(pos, Piece::Pawn, them);

    let check_mask = if checkers == 0 {
        !0u64
    } else if checkers.count_ones() == 1 {
        let checker_sq = Square::from_raw(checkers.trailing_zeros() as u8);
        line_between(king_sq.index(), checker_sq.index()) | checker_sq.bit()
    } else {
        0
    };

    Analysis {
        us,
        them,
        king_sq,
        us_occ,
        them_occ,
        occ,
        pinned,
        checkers,
        check_mask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn analysis_detects_pin_and_check_mask() {
        let pos = Position::from_fen("4r2k/8/8/8/8/8/4B3/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);

        assert!(analysis.is_pinned(sq("e2")));
        assert!(!analysis.in_check());
        assert_eq!(analysis.check_mask, !0u64);
    }

    #[test]
    fn analysis_detects_double_check() {
        let pos = Position::from_fen("4k3/8/8/8/8/5n2/4r3/4K3 w - - 0 1").unwrap();
        let analysis = analyze(&pos);

        assert!(analysis.in_check());
        assert!(analysis.double_check());
        assert_eq!(analysis.check_mask, 0);
    }
}
