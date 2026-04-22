use crate::constants::{
    BIG_HALF_DIMS, FC0_TOTAL_OUTPUTS, FC1_OUTPUTS, FC1_PADDED_INPUT_DIMS, PSQT_BUCKETS,
    SMALL_HALF_DIMS,
};
use oopsmate_core::{
    Color, EMPTY_SQUARE, MAX_POSITION_HISTORY, Move, MoveKind, Piece, Position, Square,
    color_from_code, encode_piece,
};

const MAX_DIRTY_PIECES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DirtyPiece {
    pub(crate) len: usize,
    pub(crate) piece_codes: [u8; MAX_DIRTY_PIECES],
    pub(crate) from: [Square; MAX_DIRTY_PIECES],
    pub(crate) to: [Square; MAX_DIRTY_PIECES],
}

impl DirtyPiece {
    pub(crate) const EMPTY: Self = Self {
        len: 0,
        piece_codes: [EMPTY_SQUARE; MAX_DIRTY_PIECES],
        from: [Square::NONE; MAX_DIRTY_PIECES],
        to: [Square::NONE; MAX_DIRTY_PIECES],
    };

    #[inline(always)]
    pub(crate) fn from_move(position: &Position, mv: Move) -> Self {
        let board = position.board();
        let from = mv.from();
        let to = mv.to();
        let moved = board.piece_code_at(from);
        debug_assert_ne!(moved, EMPTY_SQUARE);

        let moved_color = color_from_code(moved);
        let mut dirty = Self::EMPTY;

        match mv.kind() {
            MoveKind::Quiet | MoveKind::DoublePush => {
                dirty.push(moved, from, to);
            }
            MoveKind::Capture => {
                dirty.push(moved, from, to);
                dirty.push(board.piece_code_at(to), to, Square::NONE);
            }
            MoveKind::EnPassant => {
                dirty.push(moved, from, to);
                let capture_square = if moved_color == Color::White {
                    Square::from_raw(to.raw() - 8)
                } else {
                    Square::from_raw(to.raw() + 8)
                };
                dirty.push(
                    encode_piece(Piece::Pawn, moved_color.flip()),
                    capture_square,
                    Square::NONE,
                );
            }
            MoveKind::Castle => {
                dirty.push(moved, from, to);
                let (rook_from, rook_to) = castle_rook_squares(to);
                dirty.push(encode_piece(Piece::Rook, moved_color), rook_from, rook_to);
            }
            MoveKind::PromotionKnight
            | MoveKind::PromotionBishop
            | MoveKind::PromotionRook
            | MoveKind::PromotionQueen => {
                dirty.push(moved, from, Square::NONE);
                dirty.push(
                    encode_piece(mv.kind().promotion_piece().unwrap(), moved_color),
                    Square::NONE,
                    to,
                );
            }
            MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen => {
                dirty.push(moved, from, Square::NONE);
                dirty.push(board.piece_code_at(to), to, Square::NONE);
                dirty.push(
                    encode_piece(mv.kind().promotion_piece().unwrap(), moved_color),
                    Square::NONE,
                    to,
                );
            }
        }

        dirty
    }

    #[inline(always)]
    pub(crate) fn requires_refresh(self, perspective: Color) -> bool {
        self.len != 0 && self.piece_codes[0] == encode_piece(Piece::King, perspective)
    }

    #[inline(always)]
    fn push(&mut self, piece_code: u8, from: Square, to: Square) {
        debug_assert!(self.len < MAX_DIRTY_PIECES);
        self.piece_codes[self.len] = piece_code;
        self.from[self.len] = from;
        self.to[self.len] = to;
        self.len += 1;
    }
}

#[derive(Debug)]
pub(crate) struct AccumulatorFrame {
    pub(crate) big_accumulation: [[i16; BIG_HALF_DIMS]; 2],
    pub(crate) big_psqt: [[i32; PSQT_BUCKETS]; 2],
    pub(crate) small_accumulation: [[i16; SMALL_HALF_DIMS]; 2],
    pub(crate) small_psqt: [[i32; PSQT_BUCKETS]; 2],
    pub(crate) big_computed: [bool; 2],
    pub(crate) small_computed: [bool; 2],
    pub(crate) dirty: DirtyPiece,
}

impl AccumulatorFrame {
    fn new() -> Self {
        Self {
            big_accumulation: [[0; BIG_HALF_DIMS]; 2],
            big_psqt: [[0; PSQT_BUCKETS]; 2],
            small_accumulation: [[0; SMALL_HALF_DIMS]; 2],
            small_psqt: [[0; PSQT_BUCKETS]; 2],
            big_computed: [false; 2],
            small_computed: [false; 2],
            dirty: DirtyPiece::EMPTY,
        }
    }

    #[inline(always)]
    pub(crate) fn reset_as_root(&mut self) {
        self.big_computed = [false; 2];
        self.small_computed = [false; 2];
        self.dirty = DirtyPiece::EMPTY;
    }

    #[inline(always)]
    pub(crate) fn reset_as_child(&mut self, dirty: DirtyPiece) {
        self.big_computed = [false; 2];
        self.small_computed = [false; 2];
        self.dirty = dirty;
    }
}

#[derive(Debug)]
pub struct NnueContext {
    pub(crate) frames: Box<[AccumulatorFrame]>,
    pub(crate) depth: usize,
    pub(crate) initialized: bool,
    pub(crate) root_hash: u64,
    pub(crate) big_transformed: [u8; BIG_HALF_DIMS],
    pub(crate) small_transformed: [u8; SMALL_HALF_DIMS],
    pub(crate) fc0_out: [i32; FC0_TOTAL_OUTPUTS],
    pub(crate) fc1_in: [u8; FC1_PADDED_INPUT_DIMS],
    pub(crate) fc1_out: [i32; FC1_OUTPUTS],
    pub(crate) fc1_activated: [u8; FC1_OUTPUTS],
}

impl Default for NnueContext {
    fn default() -> Self {
        Self::new()
    }
}

impl NnueContext {
    #[must_use]
    pub fn new() -> Self {
        let mut frames = Vec::with_capacity(MAX_POSITION_HISTORY + 1);
        frames.resize_with(MAX_POSITION_HISTORY + 1, AccumulatorFrame::new);

        Self {
            frames: frames.into_boxed_slice(),
            depth: 0,
            initialized: false,
            root_hash: 0,
            big_transformed: [0; BIG_HALF_DIMS],
            small_transformed: [0; SMALL_HALF_DIMS],
            fc0_out: [0; FC0_TOTAL_OUTPUTS],
            fc1_in: [0; FC1_PADDED_INPUT_DIMS],
            fc1_out: [0; FC1_OUTPUTS],
            fc1_activated: [0; FC1_OUTPUTS],
        }
    }

    #[inline(always)]
    pub fn push_move(&mut self, position: &Position, mv: Move) {
        debug_assert!(
            self.initialized,
            "NNUE context must be reset to the root first"
        );
        debug_assert!(self.depth + 1 < self.frames.len(), "NNUE stack overflow");
        self.depth += 1;
        self.frames[self.depth].reset_as_child(DirtyPiece::from_move(position, mv));
    }

    #[inline(always)]
    pub fn pop(&mut self) {
        debug_assert!(self.depth != 0, "NNUE stack underflow");
        self.depth -= 1;
    }

    #[inline(always)]
    pub(crate) fn reset_root_state(&mut self, position: &Position) {
        self.depth = 0;
        self.initialized = true;
        self.root_hash = position.hash();
        self.frames[0].reset_as_root();
    }
}

#[inline(always)]
fn castle_rook_squares(king_to: Square) -> (Square, Square) {
    match king_to.raw() {
        6 => (Square::from_raw(7), Square::from_raw(5)),
        2 => (Square::from_raw(0), Square::from_raw(3)),
        62 => (Square::from_raw(63), Square::from_raw(61)),
        58 => (Square::from_raw(56), Square::from_raw(59)),
        _ => panic!("invalid castle move"),
    }
}

#[cfg(test)]
mod tests {
    use super::DirtyPiece;
    use oopsmate_core::{Move, MoveKind, Piece, Position, Square, encode_piece, piece_from_code};

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn dirty_piece_extracts_capture_promotion() {
        let position = Position::from_fen("1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let dirty = DirtyPiece::from_move(
            &position,
            Move::new(sq("a7"), sq("b8"), MoveKind::CapturePromotionQueen),
        );

        assert_eq!(dirty.len, 3);
        assert_eq!(
            dirty.piece_codes[0],
            encode_piece(Piece::Pawn, oopsmate_core::Color::White)
        );
        assert_eq!(dirty.from[0], sq("a7"));
        assert_eq!(dirty.to[0], Square::NONE);
        assert_eq!(dirty.from[1], sq("b8"));
        assert_eq!(dirty.to[1], Square::NONE);
        assert_eq!(
            dirty.piece_codes[2],
            encode_piece(Piece::Queen, oopsmate_core::Color::White)
        );
        assert_eq!(dirty.from[2], Square::NONE);
        assert_eq!(dirty.to[2], sq("b8"));
    }

    #[test]
    fn dirty_piece_extracts_castle_rook_move() {
        let position = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let dirty =
            DirtyPiece::from_move(&position, Move::new(sq("e1"), sq("g1"), MoveKind::Castle));

        assert_eq!(dirty.len, 2);
        assert_eq!(dirty.from[0], sq("e1"));
        assert_eq!(dirty.to[0], sq("g1"));
        assert_eq!(dirty.from[1], sq("h1"));
        assert_eq!(dirty.to[1], sq("f1"));
    }

    #[test]
    fn dirty_piece_extracts_en_passant_capture_square() {
        let position = Position::from_fen("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1").unwrap();
        let dirty = DirtyPiece::from_move(
            &position,
            Move::new(sq("e5"), sq("d6"), MoveKind::EnPassant),
        );

        assert_eq!(dirty.len, 2);
        assert_eq!(dirty.from[1], sq("d5"));
        assert_eq!(dirty.to[1], Square::NONE);
    }

    #[test]
    fn king_move_requires_refresh_for_matching_perspective() {
        let position = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let dirty =
            DirtyPiece::from_move(&position, Move::new(sq("e1"), sq("g1"), MoveKind::Castle));

        assert!(dirty.requires_refresh(oopsmate_core::Color::White));
        assert!(!dirty.requires_refresh(oopsmate_core::Color::Black));
    }

    #[test]
    fn piece_from_code_matches_core_encoding() {
        let white_queen = encode_piece(Piece::Queen, oopsmate_core::Color::White);
        assert_eq!(piece_from_code(white_queen), Piece::Queen);
    }
}
