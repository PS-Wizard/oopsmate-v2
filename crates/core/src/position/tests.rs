use super::*;
use crate::{Move, MoveKind};

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
fn null_move_round_trip_restores_state() {
    let mut pos = Position::from_fen("4k3/8/8/8/8/8/4P3/4K3 b - e3 7 12").unwrap();
    let original = pos.clone();

    pos.make_null_move();
    assert_eq!(pos.side_to_move(), Color::White);
    assert_eq!(pos.ep_square(), Square::NONE);
    assert_ne!(pos.hash(), original.hash());

    pos.unmake_null_move();
    assert_eq!(pos.hash(), original.hash());
    assert_eq!(pos.compute_hash(), original.hash());
    assert_eq!(pos.side_to_move(), original.side_to_move());
    assert_eq!(pos.castling(), original.castling());
    assert_eq!(pos.ep_square(), original.ep_square());
    assert_eq!(pos.rule50(), original.rule50());
    assert_eq!(pos.fullmove(), original.fullmove());
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
