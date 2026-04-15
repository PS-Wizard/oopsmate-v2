use oopsmate_core::{Move, MoveKind, Piece, Position};
use strikes::knight_attacks;

use crate::analysis::Analysis;
use crate::list::MoveList;
use crate::stage::{include_captures, include_quiets};
use crate::util::{piece_bb, pop_lsb};

#[inline(always)]
pub(crate) fn generate<const STAGE: u8>(pos: &Position, analysis: &Analysis, list: &mut MoveList) {
    let mut knights = piece_bb(pos, Piece::Knight, analysis.us) & !analysis.pinned;
    let empty = !analysis.occ;
    let enemy_king = pos.board().king_square(analysis.them);
    let enemy_king_bit = if enemy_king.is_valid() {
        enemy_king.bit()
    } else {
        0
    };

    while knights != 0 {
        let from = pop_lsb(&mut knights);
        let mut attacks = knight_attacks(from.index()) & analysis.check_mask & !enemy_king_bit;

        if include_captures::<STAGE>() && include_quiets::<STAGE>() {
            attacks &= !analysis.us_occ;
        } else if include_captures::<STAGE>() {
            attacks &= analysis.them_occ;
        } else {
            attacks &= empty;
        }

        while attacks != 0 {
            let to = pop_lsb(&mut attacks);
            let kind = if analysis.them_occ & to.bit() != 0 {
                MoveKind::Capture
            } else {
                MoveKind::Quiet
            };
            list.push(Move::new(from, to, kind));
        }
    }
}
