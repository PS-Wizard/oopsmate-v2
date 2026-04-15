use core::fmt;

use crate::position::Position;
use crate::types::{CastlingRights, Color, Piece, Square, encode_piece};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenError {
    MissingBoard,
    MissingSideToMove,
    MissingCastling,
    MissingEnPassant,
    InvalidBoard,
    InvalidSideToMove,
    InvalidCastling,
    InvalidEnPassant,
    InvalidHalfmove,
    InvalidFullmove,
    MissingKing(Color),
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBoard => write!(f, "missing board field"),
            Self::MissingSideToMove => write!(f, "missing side-to-move field"),
            Self::MissingCastling => write!(f, "missing castling field"),
            Self::MissingEnPassant => write!(f, "missing en-passant field"),
            Self::InvalidBoard => write!(f, "invalid board field"),
            Self::InvalidSideToMove => write!(f, "invalid side-to-move field"),
            Self::InvalidCastling => write!(f, "invalid castling field"),
            Self::InvalidEnPassant => write!(f, "invalid en-passant field"),
            Self::InvalidHalfmove => write!(f, "invalid halfmove clock"),
            Self::InvalidFullmove => write!(f, "invalid fullmove number"),
            Self::MissingKing(Color::White) => write!(f, "missing white king"),
            Self::MissingKing(Color::Black) => write!(f, "missing black king"),
        }
    }
}

impl std::error::Error for FenError {}

impl Position {
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        let mut parts = fen.split_whitespace();
        let board_part = parts.next().ok_or(FenError::MissingBoard)?;
        let stm_part = parts.next().ok_or(FenError::MissingSideToMove)?;
        let castling_part = parts.next().ok_or(FenError::MissingCastling)?;
        let ep_part = parts.next().ok_or(FenError::MissingEnPassant)?;
        let halfmove_part = parts.next();
        let fullmove_part = parts.next();

        let mut position = Position::empty();
        position.board.clear();

        let mut rank: i8 = 7;
        let mut file: u8 = 0;

        for byte in board_part.bytes() {
            match byte {
                b'/' => {
                    if file != 8 || rank == 0 {
                        return Err(FenError::InvalidBoard);
                    }
                    rank -= 1;
                    file = 0;
                }
                b'1'..=b'8' => {
                    file += byte - b'0';
                    if file > 8 {
                        return Err(FenError::InvalidBoard);
                    }
                }
                _ => {
                    if file >= 8 || rank < 0 {
                        return Err(FenError::InvalidBoard);
                    }

                    let piece_code = match byte {
                        b'P' => encode_piece(Piece::Pawn, Color::White),
                        b'N' => encode_piece(Piece::Knight, Color::White),
                        b'B' => encode_piece(Piece::Bishop, Color::White),
                        b'R' => encode_piece(Piece::Rook, Color::White),
                        b'Q' => encode_piece(Piece::Queen, Color::White),
                        b'K' => encode_piece(Piece::King, Color::White),
                        b'p' => encode_piece(Piece::Pawn, Color::Black),
                        b'n' => encode_piece(Piece::Knight, Color::Black),
                        b'b' => encode_piece(Piece::Bishop, Color::Black),
                        b'r' => encode_piece(Piece::Rook, Color::Black),
                        b'q' => encode_piece(Piece::Queen, Color::Black),
                        b'k' => encode_piece(Piece::King, Color::Black),
                        _ => return Err(FenError::InvalidBoard),
                    };

                    let square =
                        Square::from_file_rank(file, rank as u8).ok_or(FenError::InvalidBoard)?;
                    position.board.add_piece(square, piece_code);
                    file += 1;
                }
            }
        }

        if rank != 0 || file != 8 {
            return Err(FenError::InvalidBoard);
        }

        position.side_to_move = match stm_part {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenError::InvalidSideToMove),
        };

        let mut castling = CastlingRights::NONE;
        if castling_part != "-" {
            for byte in castling_part.bytes() {
                match byte {
                    b'K' => castling.0 |= CastlingRights::WHITE_KINGSIDE,
                    b'Q' => castling.0 |= CastlingRights::WHITE_QUEENSIDE,
                    b'k' => castling.0 |= CastlingRights::BLACK_KINGSIDE,
                    b'q' => castling.0 |= CastlingRights::BLACK_QUEENSIDE,
                    _ => return Err(FenError::InvalidCastling),
                }
            }
        }
        position.castling = castling;

        position.ep_square = if ep_part == "-" {
            Square::NONE
        } else {
            Square::from_algebraic(ep_part).ok_or(FenError::InvalidEnPassant)?
        };

        position.rule50 = match halfmove_part {
            Some(text) => text.parse().map_err(|_| FenError::InvalidHalfmove)?,
            None => 0,
        };

        position.fullmove = match fullmove_part {
            Some(text) => text.parse().map_err(|_| FenError::InvalidFullmove)?,
            None => 1,
        };

        if position.board.king_square(Color::White).is_none() {
            return Err(FenError::MissingKing(Color::White));
        }
        if position.board.king_square(Color::Black).is_none() {
            return Err(FenError::MissingKing(Color::Black));
        }

        position.hash = position.compute_hash();
        position.reset_history();
        Ok(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_startpos() {
        let pos = Position::from_fen(crate::position::STARTPOS_FEN).unwrap();
        assert_eq!(pos.side_to_move(), Color::White);
        assert!(pos.castling().can_castle_kingside(Color::White));
        assert!(pos.castling().can_castle_queenside(Color::Black));
    }

    #[test]
    fn rejects_missing_kings() {
        let err = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").unwrap_err();
        assert_eq!(err, FenError::MissingKing(Color::White));
    }
}
