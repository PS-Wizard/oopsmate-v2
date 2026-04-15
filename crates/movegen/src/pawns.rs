use oopsmate_core::{Color, Move, MoveKind, Piece, Position, Square};
use strikes::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks};

use crate::analysis::Analysis;
use crate::list::MoveList;
use crate::stage::{include_captures, include_promotions, include_quiets};
use crate::util::{FILE_A, FILE_H, RANK_2, RANK_7, RANK_8, piece_bb, pop_lsb};

#[inline(always)]
pub(crate) fn generate<const STAGE: u8>(pos: &Position, analysis: &Analysis, list: &mut MoveList) {
    match analysis.us {
        Color::White => generate_white::<STAGE>(pos, analysis, list),
        Color::Black => generate_black::<STAGE>(pos, analysis, list),
    }
}

#[inline(always)]
fn generate_white<const STAGE: u8>(pos: &Position, analysis: &Analysis, list: &mut MoveList) {
    let pawns = piece_bb(pos, Piece::Pawn, analysis.us);
    let unpinned = pawns & !analysis.pinned;
    let empty = !analysis.occ;
    let enemy_king = pos.board().king_square(analysis.them);
    let enemy_king_bit = if enemy_king.is_valid() {
        enemy_king.bit()
    } else {
        0
    };
    let enemies = analysis.them_occ & !enemy_king_bit;

    let single = (unpinned << 8) & empty & analysis.check_mask;
    if include_promotions::<STAGE>() {
        push_white_promotions(single & RANK_8, list, false);
    }
    if include_quiets::<STAGE>() {
        push_white_single(single & !RANK_8, list, MoveKind::Quiet);

        let double = ((((unpinned & RANK_2) << 8) & empty) << 8) & empty & analysis.check_mask;
        push_white_single(double, list, MoveKind::DoublePush);
    }

    if include_captures::<STAGE>() {
        let left = ((unpinned & !FILE_A) << 7) & enemies & analysis.check_mask;
        let right = ((unpinned & !FILE_H) << 9) & enemies & analysis.check_mask;

        let promo_left = left & RANK_8;
        let promo_right = right & RANK_8;
        if include_promotions::<STAGE>() {
            push_white_capture_promotions(promo_left, list, 7);
            push_white_capture_promotions(promo_right, list, 9);
        }

        push_white_captures(left & !RANK_8, list, 7);
        push_white_captures(right & !RANK_8, list, 9);

        generate_en_passant::<STAGE>(pos, analysis, list, pawns);
    }

    generate_white_pinned::<STAGE>(pos, analysis, list, pawns & analysis.pinned);
}

#[inline(always)]
fn generate_black<const STAGE: u8>(pos: &Position, analysis: &Analysis, list: &mut MoveList) {
    let pawns = piece_bb(pos, Piece::Pawn, analysis.us);
    let unpinned = pawns & !analysis.pinned;
    let empty = !analysis.occ;
    let enemy_king = pos.board().king_square(analysis.them);
    let enemy_king_bit = if enemy_king.is_valid() {
        enemy_king.bit()
    } else {
        0
    };
    let enemies = analysis.them_occ & !enemy_king_bit;

    let single = (unpinned >> 8) & empty & analysis.check_mask;
    if include_promotions::<STAGE>() {
        push_black_promotions(single & crate::util::RANK_1, list, false);
    }
    if include_quiets::<STAGE>() {
        push_black_single(single & !crate::util::RANK_1, list, MoveKind::Quiet);

        let double = ((((unpinned & RANK_7) >> 8) & empty) >> 8) & empty & analysis.check_mask;
        push_black_single(double, list, MoveKind::DoublePush);
    }

    if include_captures::<STAGE>() {
        let left = ((unpinned & !FILE_A) >> 9) & enemies & analysis.check_mask;
        let right = ((unpinned & !FILE_H) >> 7) & enemies & analysis.check_mask;

        let promo_left = left & crate::util::RANK_1;
        let promo_right = right & crate::util::RANK_1;
        if include_promotions::<STAGE>() {
            push_black_capture_promotions(promo_left, list, 9);
            push_black_capture_promotions(promo_right, list, 7);
        }

        push_black_captures(left & !crate::util::RANK_1, list, 9);
        push_black_captures(right & !crate::util::RANK_1, list, 7);

        generate_en_passant::<STAGE>(pos, analysis, list, pawns);
    }

    generate_black_pinned::<STAGE>(pos, analysis, list, pawns & analysis.pinned);
}

#[inline(always)]
fn generate_white_pinned<const STAGE: u8>(
    pos: &Position,
    analysis: &Analysis,
    list: &mut MoveList,
    mut pawns: u64,
) {
    let empty = !analysis.occ;
    let enemy_king = pos.board().king_square(analysis.them);
    let enemy_king_bit = if enemy_king.is_valid() {
        enemy_king.bit()
    } else {
        0
    };
    let enemies = analysis.them_occ & !enemy_king_bit;

    while pawns != 0 {
        let from = pop_lsb(&mut pawns);
        let pin_ray = analysis.pin_ray(from);

        if include_quiets::<STAGE>() {
            let one = from.raw() + 8;
            if one < 64 {
                let to = Square::from_raw(one);
                let target = to.bit();
                if target & empty & pin_ray & analysis.check_mask != 0 {
                    if target & RANK_8 != 0 {
                        if include_promotions::<STAGE>() {
                            push_promotion_set(list, from, to, false);
                        }
                    } else if include_quiets::<STAGE>() {
                        list.push(Move::new(from, to, MoveKind::Quiet));
                    }
                }
            }

            if from.bit() & RANK_2 != 0 {
                let one = Square::from_raw(from.raw() + 8);
                let two = Square::from_raw(from.raw() + 16);
                if one.bit() & empty != 0 && two.bit() & empty & pin_ray & analysis.check_mask != 0
                {
                    list.push(Move::new(from, two, MoveKind::DoublePush));
                }
            }
        }

        if include_captures::<STAGE>() {
            let mut attacks = pawn_attacks(Color::White.index(), from.index())
                & enemies
                & pin_ray
                & analysis.check_mask;
            while attacks != 0 {
                let to = pop_lsb(&mut attacks);
                if to.bit() & RANK_8 != 0 {
                    if include_promotions::<STAGE>() {
                        push_promotion_set(list, from, to, true);
                    }
                } else {
                    list.push(Move::new(from, to, MoveKind::Capture));
                }
            }
        }
    }
}

#[inline(always)]
fn generate_black_pinned<const STAGE: u8>(
    pos: &Position,
    analysis: &Analysis,
    list: &mut MoveList,
    mut pawns: u64,
) {
    let empty = !analysis.occ;
    let enemy_king = pos.board().king_square(analysis.them);
    let enemy_king_bit = if enemy_king.is_valid() {
        enemy_king.bit()
    } else {
        0
    };
    let enemies = analysis.them_occ & !enemy_king_bit;

    while pawns != 0 {
        let from = pop_lsb(&mut pawns);
        let pin_ray = analysis.pin_ray(from);

        if include_quiets::<STAGE>() {
            if from.raw() >= 8 {
                let to = Square::from_raw(from.raw() - 8);
                let target = to.bit();
                if target & empty & pin_ray & analysis.check_mask != 0 {
                    if target & crate::util::RANK_1 != 0 {
                        if include_promotions::<STAGE>() {
                            push_promotion_set(list, from, to, false);
                        }
                    } else if include_quiets::<STAGE>() {
                        list.push(Move::new(from, to, MoveKind::Quiet));
                    }
                }
            }

            if from.bit() & RANK_7 != 0 {
                let one = Square::from_raw(from.raw() - 8);
                let two = Square::from_raw(from.raw() - 16);
                if one.bit() & empty != 0 && two.bit() & empty & pin_ray & analysis.check_mask != 0
                {
                    list.push(Move::new(from, two, MoveKind::DoublePush));
                }
            }
        }

        if include_captures::<STAGE>() {
            let mut attacks = pawn_attacks(Color::Black.index(), from.index())
                & enemies
                & pin_ray
                & analysis.check_mask;
            while attacks != 0 {
                let to = pop_lsb(&mut attacks);
                if to.bit() & crate::util::RANK_1 != 0 {
                    if include_promotions::<STAGE>() {
                        push_promotion_set(list, from, to, true);
                    }
                } else {
                    list.push(Move::new(from, to, MoveKind::Capture));
                }
            }
        }
    }
}

#[inline(always)]
fn generate_en_passant<const STAGE: u8>(
    pos: &Position,
    analysis: &Analysis,
    list: &mut MoveList,
    pawns: u64,
) {
    // EP is the one pawn move where the captured piece does not sit on the
    // destination square, so normal pin/check masking is not sufficient by
    // itself. We must validate the post-EP occupancy explicitly.
    if !include_captures::<STAGE>() {
        return;
    }

    let ep_sq = pos.ep_square();
    if ep_sq.is_none() {
        return;
    }

    let ep_target = ep_sq.bit();
    let captured_sq = if analysis.us == Color::White {
        Square::from_raw(ep_sq.raw() - 8)
    } else {
        Square::from_raw(ep_sq.raw() + 8)
    };
    let captured_bit = captured_sq.bit();

    if ep_target & analysis.check_mask == 0 && captured_bit & analysis.check_mask == 0 {
        return;
    }

    let mut candidates = pawns;
    while candidates != 0 {
        let from = pop_lsb(&mut candidates);
        if pawn_attacks(analysis.us.index(), from.index()) & ep_target == 0 {
            continue;
        }

        if analysis.is_pinned(from) && ep_target & analysis.pin_ray(from) == 0 {
            continue;
        }

        // Remove both pawns from their original squares, then place the moving
        // pawn on the EP target to test the true resulting occupancy.
        let after_ep = (analysis.occ & !from.bit() & !captured_bit) | ep_target;
        if ep_exposes_check(pos, analysis, after_ep, captured_sq) {
            continue;
        }

        list.push(Move::new(from, ep_sq, MoveKind::EnPassant));
    }
}

#[inline(always)]
fn ep_exposes_check(
    pos: &Position,
    analysis: &Analysis,
    occupied: u64,
    captured_sq: Square,
) -> bool {
    let sq = analysis.king_sq.index();
    let them = analysis.them;

    if knight_attacks(sq) & piece_bb(pos, Piece::Knight, them) != 0 {
        return true;
    }

    if king_attacks(sq) & piece_bb(pos, Piece::King, them) != 0 {
        return true;
    }

    let enemy_pawns = piece_bb(pos, Piece::Pawn, them) & !captured_sq.bit();
    if pawn_attacks(them.flip().index(), sq) & enemy_pawns != 0 {
        return true;
    }

    let bishops = piece_bb(pos, Piece::Bishop, them) | piece_bb(pos, Piece::Queen, them);
    if bishop_attacks(sq, occupied) & bishops != 0 {
        return true;
    }

    let rooks = piece_bb(pos, Piece::Rook, them) | piece_bb(pos, Piece::Queen, them);
    rook_attacks(sq, occupied) & rooks != 0
}

#[inline(always)]
fn push_white_single(mut targets: u64, list: &mut MoveList, kind: MoveKind) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() - if kind == MoveKind::DoublePush { 16 } else { 8 });
        list.push(Move::new(from, to, kind));
    }
}

#[inline(always)]
fn push_black_single(mut targets: u64, list: &mut MoveList, kind: MoveKind) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() + if kind == MoveKind::DoublePush { 16 } else { 8 });
        list.push(Move::new(from, to, kind));
    }
}

#[inline(always)]
fn push_white_captures(mut targets: u64, list: &mut MoveList, delta: u8) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() - delta);
        list.push(Move::new(from, to, MoveKind::Capture));
    }
}

#[inline(always)]
fn push_black_captures(mut targets: u64, list: &mut MoveList, delta: u8) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() + delta);
        list.push(Move::new(from, to, MoveKind::Capture));
    }
}

#[inline(always)]
fn push_white_promotions(mut targets: u64, list: &mut MoveList, is_capture: bool) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() - 8);
        push_promotion_set(list, from, to, is_capture);
    }
}

#[inline(always)]
fn push_black_promotions(mut targets: u64, list: &mut MoveList, is_capture: bool) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() + 8);
        push_promotion_set(list, from, to, is_capture);
    }
}

#[inline(always)]
fn push_white_capture_promotions(mut targets: u64, list: &mut MoveList, delta: u8) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() - delta);
        push_promotion_set(list, from, to, true);
    }
}

#[inline(always)]
fn push_black_capture_promotions(mut targets: u64, list: &mut MoveList, delta: u8) {
    while targets != 0 {
        let to = pop_lsb(&mut targets);
        let from = Square::from_raw(to.raw() + delta);
        push_promotion_set(list, from, to, true);
    }
}

#[inline(always)]
fn push_promotion_set(list: &mut MoveList, from: Square, to: Square, is_capture: bool) {
    if is_capture {
        list.push(Move::new(from, to, MoveKind::CapturePromotionQueen));
        list.push(Move::new(from, to, MoveKind::CapturePromotionRook));
        list.push(Move::new(from, to, MoveKind::CapturePromotionBishop));
        list.push(Move::new(from, to, MoveKind::CapturePromotionKnight));
    } else {
        list.push(Move::new(from, to, MoveKind::PromotionQueen));
        list.push(Move::new(from, to, MoveKind::PromotionRook));
        list.push(Move::new(from, to, MoveKind::PromotionBishop));
        list.push(Move::new(from, to, MoveKind::PromotionKnight));
    }
}
