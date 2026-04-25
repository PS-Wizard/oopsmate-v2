mod make;
mod null;
#[cfg(test)]
mod tests;
mod unmake;

use crate::board::Board;
use crate::hash::{SIDE_KEY, castling_key, ep_key, piece_key, piece_key_nonempty};
use crate::types::{CastlingRights, Color, Piece, Square, encode_piece};
use crate::undo::{RepetitionStack, UndoStack};

pub const STARTPOS_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Debug)]
pub struct Position {
    pub(crate) board: Board,
    pub(crate) side_to_move: Color,
    pub(crate) castling: CastlingRights,
    pub(crate) ep_square: Square,
    pub(crate) rule50: u16,
    pub(crate) fullmove: u16,
    pub(crate) hash: u64,
    ply: u16,
    undo_stack: UndoStack,
    repetition: RepetitionStack,
}

impl Position {
    #[inline(always)]
    #[must_use]
    pub fn empty() -> Self {
        Self {
            board: Board::new(),
            side_to_move: Color::White,
            castling: CastlingRights::NONE,
            ep_square: Square::NONE,
            rule50: 0,
            fullmove: 1,
            hash: 0,
            ply: 0,
            undo_stack: UndoStack::new(),
            repetition: RepetitionStack::new(),
        }
    }

    #[inline(always)]
    #[must_use]
    pub fn startpos() -> Self {
        Self::from_fen(STARTPOS_FEN).expect("invalid start position")
    }

    #[inline(always)]
    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }

    #[inline(always)]
    #[must_use]
    pub const fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    #[inline(always)]
    #[must_use]
    pub const fn castling(&self) -> CastlingRights {
        self.castling
    }

    #[inline(always)]
    #[must_use]
    pub const fn ep_square(&self) -> Square {
        self.ep_square
    }

    #[inline(always)]
    #[must_use]
    pub const fn rule50(&self) -> u16 {
        self.rule50
    }

    #[inline(always)]
    #[must_use]
    pub const fn fullmove(&self) -> u16 {
        self.fullmove
    }

    #[inline(always)]
    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.hash
    }

    #[inline(always)]
    #[must_use]
    pub const fn ply(&self) -> u16 {
        self.ply
    }

    #[inline(always)]
    #[must_use]
    pub fn piece_at(&self, square: Square) -> Option<(Piece, Color)> {
        self.board.piece_at(square)
    }

    #[inline(always)]
    pub(crate) fn reset_history(&mut self) {
        self.undo_stack.clear();
        self.repetition.clear();
        self.repetition.push(self.hash);
        self.ply = 0;
    }

    #[must_use]
    pub fn pawn_hash(&self) -> u64 {
        let mut hash = 0u64;
        hash ^= pawn_hash_for_side(&self.board, Color::White);
        hash ^= pawn_hash_for_side(&self.board, Color::Black);
        hash
    }

    #[must_use]
    pub fn compute_hash(&self) -> u64 {
        // This is intentionally the slow full recomputation path used for setup
        // and validation. Normal move making keeps the hash incrementally.
        let mut hash = 0u64;

        for raw in 0u8..64 {
            let square = Square::from_raw(raw);
            let piece_code = self.board.piece_code_at(square);
            hash ^= piece_key(piece_code, square);
        }

        hash ^= castling_key(self.castling.0);

        if !self.ep_square.is_none() {
            hash ^= ep_key(self.ep_square);
        }

        if self.side_to_move == Color::Black {
            hash ^= SIDE_KEY;
        }

        hash
    }

    #[must_use]
    pub fn is_repetition(&self) -> bool {
        // Only positions with the same side to move can repeat, so we walk the
        // repetition history in steps of two plies instead of scanning every hash.
        if self.rule50 < 4 {
            return false;
        }

        let mut checked = 0usize;
        let mut index = self.repetition.len().saturating_sub(3);
        while checked < self.rule50 as usize && index < self.repetition.len() {
            if self.repetition.get(index) == self.hash {
                return true;
            }

            if index < 2 {
                break;
            }

            checked += 2;
            index -= 2;
        }

        false
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::startpos()
    }
}

#[inline(always)]
fn pawn_hash_for_side(board: &Board, color: Color) -> u64 {
    let piece_code = encode_piece(Piece::Pawn, color);
    let mut pawns = board.piece_bb(Piece::Pawn) & board.color_bb(color);
    let mut hash = 0u64;

    while pawns != 0 {
        let square = Square::from_raw(pawns.trailing_zeros() as u8);
        hash ^= piece_key_nonempty(piece_code, square);
        pawns &= pawns - 1;
    }

    hash
}
