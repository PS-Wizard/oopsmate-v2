use oopsmate_core::{Color, Piece, Position, Square};

pub(crate) const FILE_A: u64 = 0x0101_0101_0101_0101;
pub(crate) const FILE_H: u64 = 0x8080_8080_8080_8080;
pub(crate) const RANK_1: u64 = 0x0000_0000_0000_00ff;
pub(crate) const RANK_2: u64 = 0x0000_0000_0000_ff00;
pub(crate) const RANK_7: u64 = 0x00ff_0000_0000_0000;
pub(crate) const RANK_8: u64 = 0xff00_0000_0000_0000;

#[inline(always)]
#[must_use]
pub(crate) fn color_bb(pos: &Position, color: Color) -> u64 {
    pos.board().color_bb(color)
}

#[inline(always)]
#[must_use]
pub(crate) fn piece_bb(pos: &Position, piece: Piece, color: Color) -> u64 {
    pos.board().piece_bb(piece) & color_bb(pos, color)
}

#[inline(always)]
#[must_use]
pub(crate) fn pop_lsb(bb: &mut u64) -> Square {
    let raw = bb.trailing_zeros() as u8;
    *bb &= *bb - 1;
    Square::from_raw(raw)
}
