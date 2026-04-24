use super::Position;
use crate::moves::{Move, MoveKind};
use crate::types::{
    color_from_code, color_index_from_code, encode_piece, is_king_code, piece_from_code,
    piece_index_from_code, Color, Piece, Square,
};

impl Position {
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
        let moved = undo.moved;
        let moved_piece = piece_from_code(moved);
        let moved_color = color_from_code(moved);
        let moved_piece_index = moved_piece.index();
        let moved_color_index = moved_color.index();
        let moved_is_king = moved_piece == Piece::King;

        match kind {
            MoveKind::Quiet | MoveKind::DoublePush => {
                self.board.move_piece_known(
                    to,
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
            }
            MoveKind::Capture => {
                self.board.move_piece_known(
                    to,
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                self.board.add_piece_known(
                    to,
                    undo.captured,
                    piece_index_from_code(undo.captured),
                    color_index_from_code(undo.captured),
                    is_king_code(undo.captured),
                );
            }
            MoveKind::EnPassant => {
                self.board.move_piece_known(
                    to,
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                let capture_square = if self.side_to_move == Color::White {
                    Square::from_raw(to.raw() - 8)
                } else {
                    Square::from_raw(to.raw() + 8)
                };
                self.board.add_piece_known(
                    capture_square,
                    undo.captured,
                    piece_index_from_code(undo.captured),
                    color_index_from_code(undo.captured),
                    is_king_code(undo.captured),
                );
            }
            MoveKind::Castle => {
                self.board.move_piece_known(
                    to,
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                let (rook_from, rook_to) = match to.raw() {
                    6 => (Square::from_raw(7), Square::from_raw(5)),
                    2 => (Square::from_raw(0), Square::from_raw(3)),
                    62 => (Square::from_raw(63), Square::from_raw(61)),
                    58 => (Square::from_raw(56), Square::from_raw(59)),
                    _ => panic!("invalid castle move"),
                };
                let rook = encode_piece(Piece::Rook, moved_color);
                self.board.move_piece_known(
                    rook_to,
                    rook_from,
                    rook,
                    Piece::Rook.index(),
                    moved_color_index,
                    false,
                );
            }
            MoveKind::PromotionKnight
            | MoveKind::PromotionBishop
            | MoveKind::PromotionRook
            | MoveKind::PromotionQueen => {
                let promoted_piece = kind.promotion_piece().unwrap();
                let promoted = encode_piece(promoted_piece, moved_color);
                self.board.remove_piece_known(
                    to,
                    promoted,
                    promoted_piece.index(),
                    moved_color_index,
                    false,
                );
                self.board.add_piece_known(
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
            }
            MoveKind::CapturePromotionKnight
            | MoveKind::CapturePromotionBishop
            | MoveKind::CapturePromotionRook
            | MoveKind::CapturePromotionQueen => {
                let promoted_piece = kind.promotion_piece().unwrap();
                let promoted = encode_piece(promoted_piece, moved_color);
                self.board.remove_piece_known(
                    to,
                    promoted,
                    promoted_piece.index(),
                    moved_color_index,
                    false,
                );
                self.board.add_piece_known(
                    from,
                    moved,
                    moved_piece_index,
                    moved_color_index,
                    moved_is_king,
                );
                self.board.add_piece_known(
                    to,
                    undo.captured,
                    piece_index_from_code(undo.captured),
                    color_index_from_code(undo.captured),
                    is_king_code(undo.captured),
                );
            }
        }

        self.castling = undo.castling;
        self.ep_square = undo.ep_square;
        self.rule50 = undo.rule50;
        self.fullmove = undo.fullmove;
        self.hash = undo.hash;
    }
}
