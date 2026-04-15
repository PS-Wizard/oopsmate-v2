pub type Bitboard = u64;

pub const EMPTY_SQUARE: u8 = 0;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Color {
    #[default]
    White = 0,
    Black = 1,
}

impl Color {
    #[inline(always)]
    #[must_use]
    pub const fn flip(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Piece {
    #[default]
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl Piece {
    #[inline(always)]
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    #[inline(always)]
    #[must_use]
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::Pawn),
            1 => Some(Self::Knight),
            2 => Some(Self::Bishop),
            3 => Some(Self::Rook),
            4 => Some(Self::Queen),
            5 => Some(Self::King),
            _ => None,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Square(u8);

impl Square {
    pub const NONE: Self = Self(64);

    #[inline(always)]
    #[must_use]
    pub const fn from_raw(raw: u8) -> Self {
        Self(raw)
    }

    #[inline(always)]
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }

    #[inline(always)]
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[inline(always)]
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.0 < 64
    }

    #[inline(always)]
    #[must_use]
    pub const fn is_none(self) -> bool {
        self.0 >= 64
    }

    #[inline(always)]
    #[must_use]
    pub fn bit(self) -> Bitboard {
        debug_assert!(self.is_valid());
        1u64 << self.0
    }

    #[inline(always)]
    #[must_use]
    pub const fn file(self) -> u8 {
        self.0 & 7
    }

    #[inline(always)]
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 >> 3
    }

    #[inline(always)]
    #[must_use]
    pub const fn from_file_rank(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }

    #[must_use]
    pub fn from_algebraic(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 2 {
            return None;
        }

        let file = match bytes[0] {
            b'a'..=b'h' => bytes[0] - b'a',
            b'A'..=b'H' => bytes[0] - b'A',
            _ => return None,
        };

        let rank = match bytes[1] {
            b'1'..=b'8' => bytes[1] - b'1',
            _ => return None,
        };

        Self::from_file_rank(file, rank)
    }
}

const fn generate_rook_square_keep_masks() -> [u8; 64] {
    let mut masks = [0b1111u8; 64];
    masks[0] &= !CastlingRights::WHITE_QUEENSIDE;
    masks[7] &= !CastlingRights::WHITE_KINGSIDE;
    masks[56] &= !CastlingRights::BLACK_QUEENSIDE;
    masks[63] &= !CastlingRights::BLACK_KINGSIDE;
    masks
}

const ROOK_SQUARE_KEEP_MASKS: [u8; 64] = generate_rook_square_keep_masks();
const KING_MOVE_KEEP_MASKS: [[u8; 6]; 2] = [
    [
        0b1111,
        0b1111,
        0b1111,
        0b1111,
        0b1111,
        !(CastlingRights::WHITE_KINGSIDE | CastlingRights::WHITE_QUEENSIDE),
    ],
    [
        0b1111,
        0b1111,
        0b1111,
        0b1111,
        0b1111,
        !(CastlingRights::BLACK_KINGSIDE | CastlingRights::BLACK_QUEENSIDE),
    ],
];

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct CastlingRights(pub u8);

impl CastlingRights {
    pub const NONE: Self = Self(0);
    pub const WHITE_KINGSIDE: u8 = 1;
    pub const WHITE_QUEENSIDE: u8 = 2;
    pub const BLACK_KINGSIDE: u8 = 4;
    pub const BLACK_QUEENSIDE: u8 = 8;

    #[inline(always)]
    #[must_use]
    pub const fn can_castle_kingside(self, color: Color) -> bool {
        match color {
            Color::White => self.0 & Self::WHITE_KINGSIDE != 0,
            Color::Black => self.0 & Self::BLACK_KINGSIDE != 0,
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn can_castle_queenside(self, color: Color) -> bool {
        match color {
            Color::White => self.0 & Self::WHITE_QUEENSIDE != 0,
            Color::Black => self.0 & Self::BLACK_QUEENSIDE != 0,
        }
    }

    #[inline(always)]
    pub fn remove_color(&mut self, color: Color) {
        self.0 &= match color {
            Color::White => !(Self::WHITE_KINGSIDE | Self::WHITE_QUEENSIDE),
            Color::Black => !(Self::BLACK_KINGSIDE | Self::BLACK_QUEENSIDE),
        };
    }

    #[inline(always)]
    pub fn remove_rook_square(&mut self, square: Square) {
        if square.is_valid() {
            self.0 &= ROOK_SQUARE_KEEP_MASKS[square.index()];
        }
    }

    #[inline(always)]
    #[must_use]
    pub const fn updated_for_move(
        self,
        moved_piece: Piece,
        moved_color: Color,
        from: Square,
        to: Square,
    ) -> Self {
        Self(
            self.0
                & KING_MOVE_KEEP_MASKS[moved_color.index()][moved_piece.index()]
                & ROOK_SQUARE_KEEP_MASKS[from.index()]
                & ROOK_SQUARE_KEEP_MASKS[to.index()],
        )
    }
}

#[inline(always)]
#[must_use]
pub const fn encode_piece(piece: Piece, color: Color) -> u8 {
    1 + piece as u8 + (color as u8 * 6)
}

#[inline(always)]
#[must_use]
pub const fn piece_from_code(code: u8) -> Piece {
    match (code - 1) % 6 {
        0 => Piece::Pawn,
        1 => Piece::Knight,
        2 => Piece::Bishop,
        3 => Piece::Rook,
        4 => Piece::Queen,
        5 => Piece::King,
        _ => unreachable!(),
    }
}

#[inline(always)]
#[must_use]
pub const fn color_from_code(code: u8) -> Color {
    if code <= 6 {
        Color::White
    } else {
        Color::Black
    }
}

#[inline(always)]
#[must_use]
pub const fn decode_piece(code: u8) -> Option<(Piece, Color)> {
    if code == EMPTY_SQUARE {
        None
    } else {
        Some((piece_from_code(code), color_from_code(code)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_from_algebraic_round_trips_expected_indices() {
        assert_eq!(Square::from_algebraic("a1"), Some(Square::from_raw(0)));
        assert_eq!(Square::from_algebraic("e4"), Some(Square::from_raw(28)));
        assert_eq!(Square::from_algebraic("h8"), Some(Square::from_raw(63)));
        assert_eq!(Square::from_algebraic("z9"), None);
    }

    #[test]
    fn piece_codes_encode_and_decode() {
        let code = encode_piece(Piece::Knight, Color::Black);
        assert_eq!(decode_piece(code), Some((Piece::Knight, Color::Black)));
    }
}
