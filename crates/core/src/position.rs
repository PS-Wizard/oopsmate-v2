use crate::board::Board;
use crate::hash::{SIDE_KEY, castling_key, ep_key, piece_key, piece_key_nonempty};
use crate::moves::{Move, MoveKind};
use crate::types::{
    CastlingRights, Color, EMPTY_SQUARE, Piece, Square, color_from_code, encode_piece,
    piece_from_code,
};
use crate::undo::{RepetitionStack, Undo, UndoStack};

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

    #[inline(always)]
    fn updated_castling_rights(
        castling: CastlingRights,
        moved_piece: Piece,
        moved_color: Color,
        from: Square,
        to: Square,
    ) -> CastlingRights {
        let mut updated = castling;
        if moved_piece == Piece::King {
            updated.remove_color(moved_color);
        }
        updated.remove_rook_square(from);
        updated.remove_rook_square(to);
        updated
    }

    #[inline(always)]
    pub fn make_move(&mut self, mv: Move) {
        let from = mv.from();
        let to = mv.to();
        let kind = mv.kind();
        let moved = self.board.piece_code_at(from);
        debug_assert_ne!(moved, EMPTY_SQUARE);

        let moved_piece = piece_from_code(moved);
        let moved_color = color_from_code(moved);
        debug_assert_eq!(moved_color, self.side_to_move);

        let captured = match kind {
            MoveKind::Capture
            | MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen => self.board.piece_code_at(to),
            MoveKind::EnPassant => encode_piece(Piece::Pawn, moved_color.flip()),
            _ => EMPTY_SQUARE,
        };

        // Snapshot only the reversible state; board mutations themselves are
        // reconstructed from the move kind during unmake.
        self.undo_stack.push(Undo {
            moved,
            captured,
            castling: self.castling,
            ep_square: self.ep_square,
            rule50: self.rule50,
            fullmove: self.fullmove,
            hash: self.hash,
        });

        let old_castling = self.castling;
        if !self.ep_square.is_none() {
            self.hash ^= ep_key(self.ep_square);
        }
        self.ep_square = Square::NONE;

        self.rule50 = if moved_piece == Piece::Pawn || kind.is_capture() {
            0
        } else {
            self.rule50 + 1
        };

        match kind {
            MoveKind::Quiet => {
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece(from, to);
                self.hash ^= piece_key_nonempty(moved, to);
            }
            MoveKind::DoublePush => {
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece(from, to);
                self.hash ^= piece_key_nonempty(moved, to);
                self.ep_square = Square::from_raw((from.raw() + to.raw()) / 2);
            }
            MoveKind::Capture => {
                let removed = self.board.remove_piece(to);
                self.hash ^= piece_key_nonempty(removed, to);
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece(from, to);
                self.hash ^= piece_key_nonempty(moved, to);
            }
            MoveKind::EnPassant => {
                // The captured pawn is not on the destination square, so EP must
                // remove it from the adjacent file before moving the pawn.
                let capture_square = if moved_color == Color::White {
                    Square::from_raw(to.raw() - 8)
                } else {
                    Square::from_raw(to.raw() + 8)
                };
                let removed = self.board.remove_piece(capture_square);
                self.hash ^= piece_key_nonempty(removed, capture_square);
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece(from, to);
                self.hash ^= piece_key_nonempty(moved, to);
            }
            MoveKind::Castle => {
                // Castling is encoded as a king move; rook relocation is derived
                // from the king destination to keep Move compact.
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece(from, to);
                self.hash ^= piece_key_nonempty(moved, to);

                let (rook_from, rook_to) = match to.raw() {
                    6 => (Square::from_raw(7), Square::from_raw(5)),
                    2 => (Square::from_raw(0), Square::from_raw(3)),
                    62 => (Square::from_raw(63), Square::from_raw(61)),
                    58 => (Square::from_raw(56), Square::from_raw(59)),
                    _ => panic!("invalid castle move"),
                };
                let rook = encode_piece(Piece::Rook, moved_color);
                self.hash ^= piece_key_nonempty(rook, rook_from);
                self.board.move_piece(rook_from, rook_to);
                self.hash ^= piece_key_nonempty(rook, rook_to);
            }
            MoveKind::PromotionKnight
            | MoveKind::PromotionBishop
            | MoveKind::PromotionRook
            | MoveKind::PromotionQueen => {
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.remove_piece(from);
                let promoted = encode_piece(kind.promotion_piece().unwrap(), moved_color);
                self.board.add_piece(to, promoted);
                self.hash ^= piece_key_nonempty(promoted, to);
            }
            MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen => {
                let removed = self.board.remove_piece(to);
                self.hash ^= piece_key_nonempty(removed, to);
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.remove_piece(from);
                let promoted = encode_piece(kind.promotion_piece().unwrap(), moved_color);
                self.board.add_piece(to, promoted);
                self.hash ^= piece_key_nonempty(promoted, to);
            }
        }

        let new_castling =
            Self::updated_castling_rights(old_castling, moved_piece, moved_color, from, to);
        if new_castling != old_castling {
            self.hash ^= castling_key(old_castling.0);
            self.castling = new_castling;
            self.hash ^= castling_key(new_castling.0);
        }
        if !self.ep_square.is_none() {
            self.hash ^= ep_key(self.ep_square);
        }

        if moved_color == Color::Black {
            self.fullmove += 1;
        }

        self.side_to_move = self.side_to_move.flip();
        self.hash ^= SIDE_KEY;
        self.ply += 1;
        self.repetition.push(self.hash);
    }

    #[inline(always)]
    pub fn unmake_move(&mut self, mv: Move) {
        // Unmake restores the exact pre-move reversible state from Undo, while
        // board piece placement is reversed from the original move encoding.
        let undo = self.undo_stack.pop();
        let _ = self.repetition.pop();
        self.ply -= 1;

        self.side_to_move = self.side_to_move.flip();
        let from = mv.from();
        let to = mv.to();
        let kind = mv.kind();

        match kind {
            MoveKind::Quiet | MoveKind::DoublePush => {
                self.board.move_piece(to, from);
            }
            MoveKind::Capture => {
                self.board.move_piece(to, from);
                self.board.add_piece(to, undo.captured);
            }
            MoveKind::EnPassant => {
                self.board.move_piece(to, from);
                let capture_square = if self.side_to_move == Color::White {
                    Square::from_raw(to.raw() - 8)
                } else {
                    Square::from_raw(to.raw() + 8)
                };
                self.board.add_piece(capture_square, undo.captured);
            }
            MoveKind::Castle => {
                self.board.move_piece(to, from);
                let (rook_from, rook_to) = match to.raw() {
                    6 => (Square::from_raw(7), Square::from_raw(5)),
                    2 => (Square::from_raw(0), Square::from_raw(3)),
                    62 => (Square::from_raw(63), Square::from_raw(61)),
                    58 => (Square::from_raw(56), Square::from_raw(59)),
                    _ => panic!("invalid castle move"),
                };
                self.board.move_piece(rook_to, rook_from);
            }
            MoveKind::PromotionKnight
            | MoveKind::PromotionBishop
            | MoveKind::PromotionRook
            | MoveKind::PromotionQueen => {
                self.board.remove_piece(to);
                self.board.add_piece(from, undo.moved);
            }
            MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen => {
                self.board.remove_piece(to);
                self.board.add_piece(from, undo.moved);
                self.board.add_piece(to, undo.captured);
            }
        }

        self.castling = undo.castling;
        self.ep_square = undo.ep_square;
        self.rule50 = undo.rule50;
        self.fullmove = undo.fullmove;
        self.hash = undo.hash;
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::startpos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn startpos_has_expected_kings_and_hash() {
        let pos = Position::startpos();
        assert_eq!(pos.board.king_square(Color::White), sq("e1"));
        assert_eq!(pos.board.king_square(Color::Black), sq("e8"));
        assert_eq!(pos.hash(), pos.compute_hash());
    }

    #[test]
    fn quiet_make_unmake_round_trip_restores_hash() {
        let mut pos = Position::startpos();
        let original = pos.hash();
        let mv = Move::new(sq("g1"), sq("f3"), MoveKind::Quiet);

        pos.make_move(mv);
        pos.unmake_move(mv);

        assert_eq!(pos.hash(), original);
        assert_eq!(pos.piece_at(sq("g1")), Some((Piece::Knight, Color::White)));
        assert_eq!(pos.piece_at(sq("f3")), None);
    }

    #[test]
    fn all_special_move_kinds_round_trip() {
        let cases = [
            (
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                Move::new(sq("e1"), sq("g1"), MoveKind::Castle),
            ),
            (
                "8/8/8/3pP3/8/8/8/4K2k w - d6 0 1",
                Move::new(sq("e5"), sq("d6"), MoveKind::EnPassant),
            ),
            (
                "4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1",
                Move::new(sq("e4"), sq("d5"), MoveKind::Capture),
            ),
            (
                "4k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                Move::new(sq("a7"), sq("a8"), MoveKind::PromotionQueen),
            ),
            (
                "1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                Move::new(sq("a7"), sq("b8"), MoveKind::CapturePromotionQueen),
            ),
            (
                "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
                Move::new(sq("e2"), sq("e4"), MoveKind::DoublePush),
            ),
        ];

        for (fen, mv) in cases {
            let mut pos = Position::from_fen(fen).unwrap();
            let original = pos.clone();
            pos.make_move(mv);
            pos.unmake_move(mv);

            assert_eq!(pos.hash(), original.hash());
            assert_eq!(pos.compute_hash(), original.hash());
            assert_eq!(
                format!("{:?}", pos.board()),
                format!("{:?}", original.board())
            );
            assert_eq!(pos.side_to_move(), original.side_to_move());
            assert_eq!(pos.castling(), original.castling());
            assert_eq!(pos.ep_square(), original.ep_square());
            assert_eq!(pos.rule50(), original.rule50());
            assert_eq!(pos.fullmove(), original.fullmove());
        }
    }
}
