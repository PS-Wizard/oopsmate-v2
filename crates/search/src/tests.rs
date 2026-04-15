use std::sync::atomic::AtomicBool;

use oopsmate_core::{Move, Piece, Position};
use oopsmate_eval::PestoEval;

use crate::{SearchLimits, mate_in, search};

#[test]
fn depth_one_finds_mate_in_one() {
    let pos = Position::from_fen("7k/6Q1/6K1/8/8/8/8/8 w - - 0 1").unwrap();
    let stop = AtomicBool::new(false);
    let result = search(&pos, SearchLimits::depth(1), &stop, &PestoEval);

    assert_eq!(mate_in(result.score), Some(1));
}

#[test]
fn depth_one_prefers_winning_the_queen() {
    let pos = Position::from_fen("4k3/8/8/6b1/8/8/3q4/3RK3 w - - 0 1").unwrap();
    let stop = AtomicBool::new(false);
    let result = search(&pos, SearchLimits::depth(1), &stop, &PestoEval);

    assert_eq!(to_uci(result.best_move.unwrap()), "d1d2");
}

#[test]
fn depth_two_still_finds_mate_in_one() {
    let pos = Position::from_fen("7k/6Q1/6K1/8/8/8/8/8 w - - 0 1").unwrap();
    let stop = AtomicBool::new(false);
    let result = search(&pos, SearchLimits::depth(2), &stop, &PestoEval);

    assert_eq!(mate_in(result.score), Some(1));
}

fn to_uci(mv: Move) -> String {
    let from = mv.from();
    let to = mv.to();
    let mut text = String::with_capacity(5);
    text.push((b'a' + from.file()) as char);
    text.push((b'1' + from.rank()) as char);
    text.push((b'a' + to.file()) as char);
    text.push((b'1' + to.rank()) as char);
    if let Some(piece) = mv.kind().promotion_piece() {
        text.push(match piece {
            Piece::Knight => 'n',
            Piece::Bishop => 'b',
            Piece::Rook => 'r',
            Piece::Queen => 'q',
            Piece::Pawn | Piece::King => unreachable!(),
        });
    }
    text
}
