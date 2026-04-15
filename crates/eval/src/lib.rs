use oopsmate_core::{Board, Color, Piece, Position};

pub trait Evaluator {
    fn evaluate(&self, position: &Position) -> i32;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MaterialEval;

const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0];

impl Evaluator for MaterialEval {
    #[inline(always)]
    fn evaluate(&self, position: &Position) -> i32 {
        let white = material_score(position.board(), Color::White);
        let black = material_score(position.board(), Color::Black);
        let score = white - black;

        if position.side_to_move() == Color::White {
            score
        } else {
            -score
        }
    }
}

#[inline(always)]
fn material_score(board: &Board, color: Color) -> i32 {
    let color_bb = board.color_bb(color);
    let mut total = 0;

    for piece in [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ] {
        let count = (board.piece_bb(piece) & color_bb).count_ones() as i32;
        total += count * PIECE_VALUES[piece.index()];
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startpos_is_materially_equal() {
        let pos = Position::startpos();
        assert_eq!(MaterialEval.evaluate(&pos), 0);
    }

    #[test]
    fn score_is_from_side_to_move_perspective() {
        let white = Position::from_fen("4k3/8/8/8/8/8/8/Q3K3 w - - 0 1").unwrap();
        let black = Position::from_fen("4k3/8/8/8/8/8/8/Q3K3 b - - 0 1").unwrap();

        assert!(MaterialEval.evaluate(&white) > 0);
        assert!(MaterialEval.evaluate(&black) < 0);
    }
}
