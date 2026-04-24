use super::Position;
use crate::hash::{ep_key, SIDE_KEY};
use crate::types::{Color, Square, EMPTY_SQUARE};
use crate::undo::Undo;

impl Position {
    #[inline(always)]
    pub fn make_null_move(&mut self) {
        self.undo_stack.push(Undo {
            moved: EMPTY_SQUARE,
            captured: EMPTY_SQUARE,
            castling: self.castling,
            ep_square: self.ep_square,
            rule50: self.rule50,
            fullmove: self.fullmove,
            hash: self.hash,
        });

        if !self.ep_square.is_none() {
            self.hash ^= ep_key(self.ep_square);
            self.ep_square = Square::NONE;
        }

        self.rule50 += 1;
        if self.side_to_move == Color::Black {
            self.fullmove += 1;
        }
        self.side_to_move = self.side_to_move.flip();
        self.hash ^= SIDE_KEY;
        self.ply += 1;
        self.repetition.push(self.hash);
    }

    #[inline(always)]
    pub fn unmake_null_move(&mut self) {
        let undo = self.undo_stack.pop();
        let _ = self.repetition.pop();
        self.ply -= 1;
        self.side_to_move = self.side_to_move.flip();
        self.castling = undo.castling;
        self.ep_square = undo.ep_square;
        self.rule50 = undo.rule50;
        self.fullmove = undo.fullmove;
        self.hash = undo.hash;
    }
}
