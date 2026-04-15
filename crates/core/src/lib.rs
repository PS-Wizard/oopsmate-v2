pub mod board;
pub mod fen;
pub mod hash;
pub mod moves;
pub mod position;
pub mod types;
pub mod undo;

pub use board::Board;
pub use fen::FenError;
pub use moves::{Move, MoveKind};
pub use position::{Position, STARTPOS_FEN};
pub use types::{
    Bitboard, CastlingRights, Color, EMPTY_SQUARE, Piece, Square, color_from_code, decode_piece,
    encode_piece, piece_from_code,
};
pub use undo::{MAX_POSITION_HISTORY, Undo};

pub const ENGINE_NAME: &str = "oopsmate-v2";

#[must_use]
pub const fn engine_name() -> &'static str {
    ENGINE_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_name_is_stable() {
        assert_eq!(engine_name(), "oopsmate-v2");
    }
}
