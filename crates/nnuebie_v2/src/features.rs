use crate::constants::{FEATURE_DIMS, MAX_ACTIVE_FEATURES};
use oopsmate_core::{
    Board, Color, EMPTY_SQUARE, Piece, Position, color_from_code, piece_from_code,
};

const NO_FLIP: usize = 0;
const FLIP_HORIZONTAL: usize = 7;
const FLIP_VERTICAL: usize = 56;
const FLIP_BOTH: usize = 63;

const PS_W_PAWN: usize = 0 * 64;
const PS_B_PAWN: usize = 1 * 64;
const PS_W_KNIGHT: usize = 2 * 64;
const PS_B_KNIGHT: usize = 3 * 64;
const PS_W_BISHOP: usize = 4 * 64;
const PS_B_BISHOP: usize = 5 * 64;
const PS_W_ROOK: usize = 6 * 64;
const PS_B_ROOK: usize = 7 * 64;
const PS_W_QUEEN: usize = 8 * 64;
const PS_B_QUEEN: usize = 9 * 64;
const PS_KING: usize = 10 * 64;
const PS_NB: usize = 11 * 64;

const fn bucket(v: usize) -> usize {
    v * PS_NB
}

#[rustfmt::skip]
const ORIENT_TBL: [[usize; 64]; 2] = [
    [
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
        FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, FLIP_HORIZONTAL, NO_FLIP, NO_FLIP, NO_FLIP, NO_FLIP,
    ],
    [
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
        FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_BOTH, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL, FLIP_VERTICAL,
    ],
];

#[rustfmt::skip]
const KING_BUCKETS: [[usize; 64]; 2] = [
    [
        bucket(28), bucket(29), bucket(30), bucket(31), bucket(31), bucket(30), bucket(29), bucket(28),
        bucket(24), bucket(25), bucket(26), bucket(27), bucket(27), bucket(26), bucket(25), bucket(24),
        bucket(20), bucket(21), bucket(22), bucket(23), bucket(23), bucket(22), bucket(21), bucket(20),
        bucket(16), bucket(17), bucket(18), bucket(19), bucket(19), bucket(18), bucket(17), bucket(16),
        bucket(12), bucket(13), bucket(14), bucket(15), bucket(15), bucket(14), bucket(13), bucket(12),
        bucket(8),  bucket(9),  bucket(10), bucket(11), bucket(11), bucket(10), bucket(9),  bucket(8),
        bucket(4),  bucket(5),  bucket(6),  bucket(7),  bucket(7),  bucket(6),  bucket(5),  bucket(4),
        bucket(0),  bucket(1),  bucket(2),  bucket(3),  bucket(3),  bucket(2),  bucket(1),  bucket(0),
    ],
    [
        bucket(0),  bucket(1),  bucket(2),  bucket(3),  bucket(3),  bucket(2),  bucket(1),  bucket(0),
        bucket(4),  bucket(5),  bucket(6),  bucket(7),  bucket(7),  bucket(6),  bucket(5),  bucket(4),
        bucket(8),  bucket(9),  bucket(10), bucket(11), bucket(11), bucket(10), bucket(9),  bucket(8),
        bucket(12), bucket(13), bucket(14), bucket(15), bucket(15), bucket(14), bucket(13), bucket(12),
        bucket(16), bucket(17), bucket(18), bucket(19), bucket(19), bucket(18), bucket(17), bucket(16),
        bucket(20), bucket(21), bucket(22), bucket(23), bucket(23), bucket(22), bucket(21), bucket(20),
        bucket(24), bucket(25), bucket(26), bucket(27), bucket(27), bucket(26), bucket(25), bucket(24),
        bucket(28), bucket(29), bucket(30), bucket(31), bucket(31), bucket(30), bucket(29), bucket(28),
    ],
];

#[inline(always)]
fn piece_square_index(perspective: usize, piece: Piece, color: Color) -> usize {
    match (perspective, color, piece) {
        (0, Color::White, Piece::Pawn) => PS_W_PAWN,
        (0, Color::White, Piece::Knight) => PS_W_KNIGHT,
        (0, Color::White, Piece::Bishop) => PS_W_BISHOP,
        (0, Color::White, Piece::Rook) => PS_W_ROOK,
        (0, Color::White, Piece::Queen) => PS_W_QUEEN,
        (0, Color::White, Piece::King) => PS_KING,
        (0, Color::Black, Piece::Pawn) => PS_B_PAWN,
        (0, Color::Black, Piece::Knight) => PS_B_KNIGHT,
        (0, Color::Black, Piece::Bishop) => PS_B_BISHOP,
        (0, Color::Black, Piece::Rook) => PS_B_ROOK,
        (0, Color::Black, Piece::Queen) => PS_B_QUEEN,
        (0, Color::Black, Piece::King) => PS_KING,
        (1, Color::White, Piece::Pawn) => PS_B_PAWN,
        (1, Color::White, Piece::Knight) => PS_B_KNIGHT,
        (1, Color::White, Piece::Bishop) => PS_B_BISHOP,
        (1, Color::White, Piece::Rook) => PS_B_ROOK,
        (1, Color::White, Piece::Queen) => PS_B_QUEEN,
        (1, Color::White, Piece::King) => PS_KING,
        (1, Color::Black, Piece::Pawn) => PS_W_PAWN,
        (1, Color::Black, Piece::Knight) => PS_W_KNIGHT,
        (1, Color::Black, Piece::Bishop) => PS_W_BISHOP,
        (1, Color::Black, Piece::Rook) => PS_W_ROOK,
        (1, Color::Black, Piece::Queen) => PS_W_QUEEN,
        (1, Color::Black, Piece::King) => PS_KING,
        _ => unreachable!("invalid perspective"),
    }
}

#[inline(always)]
pub fn enumerate_active_features(
    position: &Position,
    out: &mut [[u32; MAX_ACTIVE_FEATURES]; 2],
    lengths: &mut [usize; 2],
) -> usize {
    lengths.fill(0);

    let board = position.board();
    let white_king = board.king_square(Color::White).raw() as usize;
    let black_king = board.king_square(Color::Black).raw() as usize;

    push_perspective_features(board, 0, white_king, &mut out[0], &mut lengths[0]);
    push_perspective_features(board, 1, black_king, &mut out[1], &mut lengths[1]);

    debug_assert_eq!(lengths[0], lengths[1]);
    debug_assert!(lengths[0] <= MAX_ACTIVE_FEATURES);
    lengths[0]
}

fn push_perspective_features(
    board: &Board,
    perspective: usize,
    king_square: usize,
    out: &mut [u32; MAX_ACTIVE_FEATURES],
    len: &mut usize,
) {
    let orient = ORIENT_TBL[perspective][king_square];
    let king_bucket = KING_BUCKETS[perspective][king_square];

    for (square, &piece_code) in board.squares().iter().enumerate() {
        if piece_code == EMPTY_SQUARE {
            continue;
        }

        let piece = piece_from_code(piece_code);
        let color = color_from_code(piece_code);
        let index = (square ^ orient) + piece_square_index(perspective, piece, color) + king_bucket;

        debug_assert!(index < FEATURE_DIMS);
        out[*len] = index as u32;
        *len += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::enumerate_active_features;
    use crate::constants::MAX_ACTIVE_FEATURES;
    use oopsmate_core::Position;

    #[test]
    fn startpos_enumerates_32_features_per_perspective() {
        let position = Position::startpos();
        let mut indices = [[0u32; MAX_ACTIVE_FEATURES]; 2];
        let mut lengths = [0usize; 2];

        let piece_count = enumerate_active_features(&position, &mut indices, &mut lengths);

        assert_eq!(piece_count, 32);
        assert_eq!(lengths, [32, 32]);
    }
}
