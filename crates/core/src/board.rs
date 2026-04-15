use crate::types::{
    Bitboard, Color, EMPTY_SQUARE, Piece, Square, color_from_code, decode_piece, piece_from_code,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    pieces: [Bitboard; 6],
    colors: [Bitboard; 2],
    occupied: Bitboard,
    squares: [u8; 64],
    king_sq: [Square; 2],
}

impl Board {
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            pieces: [0; 6],
            colors: [0; 2],
            occupied: 0,
            squares: [EMPTY_SQUARE; 64],
            king_sq: [Square::NONE; 2],
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    #[inline(always)]
    #[must_use]
    pub const fn piece_bb(&self, piece: Piece) -> Bitboard {
        self.pieces[piece.index()]
    }

    #[inline(always)]
    #[must_use]
    pub const fn color_bb(&self, color: Color) -> Bitboard {
        self.colors[color.index()]
    }

    #[inline(always)]
    #[must_use]
    pub const fn occupied(&self) -> Bitboard {
        self.occupied
    }

    #[inline(always)]
    #[must_use]
    pub const fn piece_code_at(&self, square: Square) -> u8 {
        self.squares[square.index()]
    }

    #[inline(always)]
    #[must_use]
    pub fn piece_at(&self, square: Square) -> Option<(Piece, Color)> {
        decode_piece(self.piece_code_at(square))
    }

    #[inline(always)]
    #[must_use]
    pub const fn king_square(&self, color: Color) -> Square {
        self.king_sq[color.index()]
    }

    #[inline(always)]
    #[must_use]
    pub const fn squares(&self) -> &[u8; 64] {
        &self.squares
    }

    #[inline(always)]
    pub fn add_piece(&mut self, square: Square, piece_code: u8) {
        debug_assert!(square.is_valid());
        debug_assert_eq!(self.squares[square.index()], EMPTY_SQUARE);
        debug_assert_ne!(piece_code, EMPTY_SQUARE);

        let piece = piece_from_code(piece_code);
        let color = color_from_code(piece_code);
        let bit = square.bit();

        self.pieces[piece.index()] |= bit;
        self.colors[color.index()] |= bit;
        self.occupied |= bit;
        self.squares[square.index()] = piece_code;

        if piece == Piece::King {
            self.king_sq[color.index()] = square;
        }
    }

    #[inline(always)]
    pub fn remove_piece(&mut self, square: Square) -> u8 {
        debug_assert!(square.is_valid());

        let piece_code = self.squares[square.index()];
        if piece_code == EMPTY_SQUARE {
            return EMPTY_SQUARE;
        }

        let piece = piece_from_code(piece_code);
        let color = color_from_code(piece_code);
        let bit = square.bit();

        self.pieces[piece.index()] &= !bit;
        self.colors[color.index()] &= !bit;
        self.occupied &= !bit;
        self.squares[square.index()] = EMPTY_SQUARE;

        if piece == Piece::King {
            self.king_sq[color.index()] = Square::NONE;
        }

        piece_code
    }

    #[inline(always)]
    pub fn move_piece(&mut self, from: Square, to: Square) -> u8 {
        debug_assert!(from.is_valid());
        debug_assert!(to.is_valid());

        let piece_code = self.squares[from.index()];
        debug_assert_ne!(piece_code, EMPTY_SQUARE);
        debug_assert_eq!(self.squares[to.index()], EMPTY_SQUARE);

        let piece = piece_from_code(piece_code);
        let color = color_from_code(piece_code);
        let from_bit = from.bit();
        let to_bit = to.bit();
        let from_to_mask = from_bit | to_bit;

        self.pieces[piece.index()] ^= from_to_mask;
        self.colors[color.index()] ^= from_to_mask;
        self.occupied ^= from_to_mask;
        self.squares[from.index()] = EMPTY_SQUARE;
        self.squares[to.index()] = piece_code;

        if piece == Piece::King {
            self.king_sq[color.index()] = to;
        }

        piece_code
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Piece, encode_piece};

    #[test]
    fn add_move_remove_piece_keeps_board_consistent() {
        let mut board = Board::new();
        let e2 = Square::from_algebraic("e2").unwrap();
        let e4 = Square::from_algebraic("e4").unwrap();
        let wk = Square::from_algebraic("e1").unwrap();

        board.add_piece(wk, encode_piece(Piece::King, Color::White));
        board.add_piece(e2, encode_piece(Piece::Pawn, Color::White));

        assert_eq!(board.king_square(Color::White), wk);
        assert_eq!(board.occupied().count_ones(), 2);

        board.move_piece(e2, e4);
        assert_eq!(board.piece_at(e4), Some((Piece::Pawn, Color::White)));
        assert_eq!(board.piece_at(e2), None);

        board.remove_piece(e4);
        assert_eq!(board.occupied().count_ones(), 1);
    }
}
