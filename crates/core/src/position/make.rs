use super::Position;
use crate::hash::{castling_key, ep_key, piece_key_nonempty, SIDE_KEY};
use crate::moves::{Move, MoveKind};
use crate::types::{
    color_from_code, encode_piece, piece_from_code, Color, Piece, Square, EMPTY_SQUARE,
};
use crate::undo::Undo;

impl Position {
    #[inline(always)]
    pub fn make_move(&mut self, mv: Move) {
        let from = mv.from();
        let to = mv.to();
        let kind = mv.kind();
        let moved = self.board.piece_code_at(from);
        debug_assert_ne!(moved, EMPTY_SQUARE);

        let moved_piece = piece_from_code(moved);
        let moved_color = color_from_code(moved);
        let moved_piece_index = moved_piece.index();
        let moved_color_index = moved_color.index();
        let moved_is_king = moved_piece == Piece::King;
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
                self.board.move_piece_known(
                    from,
                    to,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                self.hash ^= piece_key_nonempty(moved, to);
            }
            MoveKind::DoublePush => {
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece_known(
                    from,
                    to,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                self.hash ^= piece_key_nonempty(moved, to);
                self.ep_square = Square::from_raw((from.raw() + to.raw()) / 2);
            }
            MoveKind::Capture => {
                let removed = self.board.remove_piece(to);
                self.hash ^= piece_key_nonempty(removed, to);
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece_known(
                    from,
                    to,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
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
                self.board.move_piece_known(
                    from,
                    to,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                self.hash ^= piece_key_nonempty(moved, to);
            }
            MoveKind::Castle => {
                // Castling is encoded as a king move; rook relocation is derived
                // from the king destination to keep Move compact.
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.move_piece_known(
                    from,
                    to,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
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
                self.board.move_piece_known(
                    rook_from,
                    rook_to,
                    rook,
                    Piece::Rook.index(),
                    moved_color_index,
                    false,
                );
                self.hash ^= piece_key_nonempty(rook, rook_to);
            }
            MoveKind::PromotionKnight
            | MoveKind::PromotionBishop
            | MoveKind::PromotionRook
            | MoveKind::PromotionQueen => {
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.remove_piece_known(
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                let promoted_piece = kind.promotion_piece().unwrap();
                let promoted = encode_piece(promoted_piece, moved_color);
                self.board.add_piece_known(
                    to,
                    promoted,
                    promoted_piece.index(),
                    moved_color_index,
                    false,
                );
                self.hash ^= piece_key_nonempty(promoted, to);
            }
            MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen => {
                let removed = self.board.remove_piece(to);
                self.hash ^= piece_key_nonempty(removed, to);
                self.hash ^= piece_key_nonempty(moved, from);
                self.board.remove_piece_known(
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                let promoted_piece = kind.promotion_piece().unwrap();
                let promoted = encode_piece(promoted_piece, moved_color);
                self.board.add_piece_known(
                    to,
                    promoted,
                    promoted_piece.index(),
                    moved_color_index,
                    false,
                );
                self.hash ^= piece_key_nonempty(promoted, to);
            }
        }

        let new_castling = old_castling.updated_for_move(moved_piece, moved_color, from, to);
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
}
