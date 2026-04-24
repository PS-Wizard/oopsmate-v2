use std::sync::atomic::AtomicBool;

use oopsmate_core::{Move, Piece, Position};
use oopsmate_eval::PestoEval;
use oopsmate_memory::{Bound, SearchMemory};

use crate::{SearchLimits, mate_in, search};

#[test]
fn depth_one_finds_mate_in_one() {
    let pos = Position::from_fen("7k/6Q1/6K1/8/8/8/8/8 w - - 0 1").unwrap();
    let stop = AtomicBool::new(false);
    let mut memory = SearchMemory::new(1);
    let result = search(&pos, SearchLimits::depth(1), &stop, &mut memory, &PestoEval);

    assert_eq!(mate_in(result.score), Some(1));
}

#[test]
fn depth_one_prefers_winning_the_queen() {
    let pos = Position::from_fen("4k3/8/8/6b1/8/8/3q4/3RK3 w - - 0 1").unwrap();
    let stop = AtomicBool::new(false);
    let mut memory = SearchMemory::new(1);
    let result = search(&pos, SearchLimits::depth(1), &stop, &mut memory, &PestoEval);

    assert_eq!(to_uci(result.best_move.unwrap()), "d1d2");
}

#[test]
fn depth_two_still_finds_mate_in_one() {
    let pos = Position::from_fen("7k/6Q1/6K1/8/8/8/8/8 w - - 0 1").unwrap();
    let stop = AtomicBool::new(false);
    let mut memory = SearchMemory::new(1);
    let result = search(&pos, SearchLimits::depth(2), &stop, &mut memory, &PestoEval);

    assert_eq!(mate_in(result.score), Some(1));
}

#[test]
fn depth_one_avoids_poisoned_queen_capture_with_qsearch() {
    let pos = Position::from_fen("7k/8/8/8/8/4b3/3q4/3Q3K w - - 0 1").unwrap();
    let stop = AtomicBool::new(false);
    let mut memory = SearchMemory::new(1);
    let result = search(&pos, SearchLimits::depth(1), &stop, &mut memory, &PestoEval);

    assert_ne!(to_uci(result.best_move.unwrap()), "d1d2");
    assert!(result.score < 0);
}

#[test]
fn root_tt_move_is_validated_before_use() {
    let pos = Position::startpos();
    let stop = AtomicBool::new(false);
    let mut memory = SearchMemory::new(1);
    let bogus = Move::new(
        oopsmate_core::Square::from_algebraic("a3").unwrap(),
        oopsmate_core::Square::from_algebraic("a4").unwrap(),
        oopsmate_core::MoveKind::Quiet,
    );

    memory
        .tt
        .store(pos.hash(), 0, bogus, 0, i16::MIN, 1, Bound::Exact);

    let result = search(&pos, SearchLimits::depth(1), &stop, &mut memory, &PestoEval);

    assert_ne!(result.best_move, Some(bogus));
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
