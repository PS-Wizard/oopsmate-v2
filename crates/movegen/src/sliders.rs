use oopsmate_core::{Move, MoveKind, Piece, Position};
use strikes::{bishop_attacks, queen_attacks, rook_attacks};

use crate::analysis::Analysis;
use crate::list::MoveList;
use crate::stage::{include_captures, include_quiets};
use crate::util::{piece_bb, pop_lsb};

#[inline(always)]
pub(crate) fn generate<const STAGE: u8>(pos: &Position, analysis: &Analysis, list: &mut MoveList) {
    generate_bishops::<STAGE>(piece_bb(pos, Piece::Bishop, analysis.us), analysis, list);
    generate_rooks::<STAGE>(piece_bb(pos, Piece::Rook, analysis.us), analysis, list);
    generate_queens::<STAGE>(piece_bb(pos, Piece::Queen, analysis.us), analysis, list);
}

// These are specialized per-piece instead of using a shared function-pointer
// helper because dispatch in this loop showed up in profiling.
macro_rules! define_slider_generator {
    ($name:ident, $attack_fn:ident) => {
        #[inline(always)]
        fn $name<const STAGE: u8>(mut pieces: u64, analysis: &Analysis, list: &mut MoveList) {
            let empty = !analysis.occ;

            while pieces != 0 {
                let from = pop_lsb(&mut pieces);
                let mut attacks =
                    $attack_fn(from.index(), analysis.occ) & analysis.check_mask & !analysis.us_occ;

                if analysis.is_pinned(from) {
                    attacks &= analysis.pin_ray(from);
                }

                if include_captures::<STAGE>() && include_quiets::<STAGE>() {
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
    };
}

define_slider_generator!(generate_bishops, bishop_attacks);
define_slider_generator!(generate_rooks, rook_attacks);
define_slider_generator!(generate_queens, queen_attacks);
