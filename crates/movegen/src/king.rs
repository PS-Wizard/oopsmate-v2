use oopsmate_core::{Color, Move, MoveKind, Position, Square};

use crate::analysis::Analysis;
use crate::attacks::is_square_attacked_with_occ;
use crate::list::MoveList;
use crate::stage::include_quiets;
use strikes::king_attacks;

#[inline(always)]
pub(crate) fn generate<const STAGE: u8>(pos: &Position, analysis: &Analysis, list: &mut MoveList) {
    let from = analysis.king_sq;
    let from_bit = from.bit();
    let enemy_king = pos.board().king_square(analysis.them);
    let enemy_king_bit = if enemy_king.is_valid() {
        enemy_king.bit()
    } else {
        0
    };

    let mut targets = king_attacks(from.index()) & !analysis.us_occ & !enemy_king_bit;
    while targets != 0 {
        let to = Square::from_raw(targets.trailing_zeros() as u8);
        targets &= targets - 1;

        let is_capture = analysis.them_occ & to.bit() != 0;
        if is_capture {
            if STAGE == crate::GenerationStage::Quiets as u8 {
                continue;
            }
        } else if !include_quiets::<STAGE>() {
            continue;
        }

        // Re-test attacks with the king on its destination square because king
        // moves cannot rely on the precomputed node-wide constraints.
        let occ_after = (analysis.occ & !from_bit & !to.bit()) | to.bit();
        if !is_square_attacked_with_occ(pos, to, analysis.them, occ_after) {
            let kind = if is_capture {
                MoveKind::Capture
            } else {
                MoveKind::Quiet
            };
            list.push(Move::new(from, to, kind));
        }
    }

    if include_quiets::<STAGE>() && !analysis.in_check() {
        generate_castling(pos, analysis, list);
    }
}

#[inline(always)]
fn generate_castling(pos: &Position, analysis: &Analysis, list: &mut MoveList) {
    // Castling is generated only from the quiet stages. Rights alone are not
    // enough here; we also verify rook presence so corrupted rights never leak
    // a bogus castle move into perft/search.
    let occupied = analysis.occ;

    match analysis.us {
        Color::White => {
            if pos.castling().can_castle_kingside(Color::White)
                && occupied & 0x0000_0000_0000_0060 == 0
                && pos.piece_at(Square::from_raw(7))
                    == Some((oopsmate_core::Piece::Rook, Color::White))
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(5), analysis.them)
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(6), analysis.them)
            {
                list.push(Move::new(
                    analysis.king_sq,
                    Square::from_raw(6),
                    MoveKind::Castle,
                ));
            }

            if pos.castling().can_castle_queenside(Color::White)
                && occupied & 0x0000_0000_0000_000e == 0
                && pos.piece_at(Square::from_raw(0))
                    == Some((oopsmate_core::Piece::Rook, Color::White))
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(3), analysis.them)
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(2), analysis.them)
            {
                list.push(Move::new(
                    analysis.king_sq,
                    Square::from_raw(2),
                    MoveKind::Castle,
                ));
            }
        }
        Color::Black => {
            if pos.castling().can_castle_kingside(Color::Black)
                && occupied & 0x6000_0000_0000_0000 == 0
                && pos.piece_at(Square::from_raw(63))
                    == Some((oopsmate_core::Piece::Rook, Color::Black))
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(61), analysis.them)
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(62), analysis.them)
            {
                list.push(Move::new(
                    analysis.king_sq,
                    Square::from_raw(62),
                    MoveKind::Castle,
                ));
            }

            if pos.castling().can_castle_queenside(Color::Black)
                && occupied & 0x0e00_0000_0000_0000 == 0
                && pos.piece_at(Square::from_raw(56))
                    == Some((oopsmate_core::Piece::Rook, Color::Black))
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(59), analysis.them)
                && !crate::attacks::is_square_attacked(pos, Square::from_raw(58), analysis.them)
            {
                list.push(Move::new(
                    analysis.king_sq,
                    Square::from_raw(58),
                    MoveKind::Castle,
                ));
            }
        }
    }
}
