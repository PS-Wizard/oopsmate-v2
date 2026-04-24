use oopsmate_core::{Move, MoveKind, Piece, Position};
use strikes::{
    bishop_attacks, king_attacks, knight_attacks, line_through, pawn_attacks, rook_attacks,
};

#[inline(always)]
#[must_use]
pub fn might_give_check(pos: &Position, mv: Move) -> bool {
    if mv.kind() == MoveKind::Castle {
        return true;
    }

    let Some((piece, us)) = pos.piece_at(mv.from()) else {
        return true;
    };

    let enemy_king = pos.board().king_square(us.flip());
    let enemy_king_bit = enemy_king.bit();
    let from = mv.from();
    let to = mv.to();

    if line_through(enemy_king.index(), from.index()) != 0 {
        return true;
    }

    let moved_piece = mv.kind().promotion_piece().unwrap_or(piece);
    let to_idx = to.index();
    match moved_piece {
        Piece::Pawn => pawn_attacks(us.index(), to_idx) & enemy_king_bit != 0,
        Piece::Knight => knight_attacks(to_idx) & enemy_king_bit != 0,
        Piece::Bishop => bishop_attacks(to_idx, moved_occupancy(pos, mv)) & enemy_king_bit != 0,
        Piece::Rook => rook_attacks(to_idx, moved_occupancy(pos, mv)) & enemy_king_bit != 0,
        Piece::Queen => {
            let occ = moved_occupancy(pos, mv);
            (bishop_attacks(to_idx, occ) | rook_attacks(to_idx, occ)) & enemy_king_bit != 0
        }
        Piece::King => king_attacks(to_idx) & enemy_king_bit != 0,
    }
}

#[inline(always)]
fn moved_occupancy(pos: &Position, mv: Move) -> u64 {
    (pos.board().occupied() & !mv.from().bit()) | mv.to().bit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oopsmate_core::{MoveKind, Square};

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn detects_direct_knight_check() {
        let pos = Position::from_fen("8/8/8/8/7k/8/3N4/4K3 w - - 0 1").unwrap();
        let mv = Move::new(sq("d2"), sq("f3"), MoveKind::Quiet);

        assert!(might_give_check(&pos, mv));
    }

    #[test]
    fn detects_possible_discovered_check() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/4B3/4K3 w - - 0 1").unwrap();
        let mv = Move::new(sq("e2"), sq("d3"), MoveKind::Quiet);

        assert!(might_give_check(&pos, mv));
    }

    #[test]
    fn rejects_plain_non_check() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/3N4/4K3 w - - 0 1").unwrap();
        let mv = Move::new(sq("d2"), sq("b3"), MoveKind::Quiet);

        assert!(!might_give_check(&pos, mv));
    }
}
