use oopsmate_core::{Move, MoveKind, Position, Square};

use crate::{
    KIWIPETE, MoveList, POSITION_3, POSITION_4, POSITION_5, POSITION_6, STARTPOS, generate_all,
    generate_captures_promotions, generate_quiets, is_legal, perft,
};

fn sq(text: &str) -> Square {
    Square::from_algebraic(text).unwrap()
}

fn all_kinds() -> [MoveKind; 13] {
    [
        MoveKind::Quiet,
        MoveKind::DoublePush,
        MoveKind::Castle,
        MoveKind::EnPassant,
        MoveKind::Capture,
        MoveKind::PromotionKnight,
        MoveKind::PromotionBishop,
        MoveKind::PromotionRook,
        MoveKind::PromotionQueen,
        MoveKind::CapturePromotionKnight,
        MoveKind::CapturePromotionBishop,
        MoveKind::CapturePromotionRook,
        MoveKind::CapturePromotionQueen,
    ]
}

fn generated_moves(pos: &Position) -> Vec<Move> {
    let mut list = MoveList::new();
    generate_all(pos, &mut list);
    let mut moves = list.as_slice().to_vec();
    moves.sort_by_key(|mv| mv.0);
    moves
}

fn legal_oracle(pos: &Position) -> Vec<Move> {
    let mut moves = Vec::new();

    for from in 0u8..64 {
        for to in 0u8..64 {
            if from == to {
                continue;
            }

            let from_sq = Square::from_raw(from);
            let to_sq = Square::from_raw(to);
            for kind in all_kinds() {
                let mv = Move::new(from_sq, to_sq, kind);
                if is_legal(pos, mv) {
                    moves.push(mv);
                }
            }
        }
    }

    moves.sort_by_key(|mv| mv.0);
    moves.dedup_by_key(|mv| mv.0);
    moves
}

#[test]
fn staged_generation_splits_quiets_and_captures_cleanly() {
    let pos = Position::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
    let mut all = MoveList::new();
    let mut captures = MoveList::new();
    let mut quiets = MoveList::new();

    generate_all(&pos, &mut all);
    generate_captures_promotions(&pos, &mut captures);
    generate_quiets(&pos, &mut quiets);

    for &mv in captures.as_slice() {
        assert!(mv.is_capture() || mv.is_promotion() || mv.kind() == MoveKind::EnPassant);
        assert!(all.contains(mv));
    }

    for &mv in quiets.as_slice() {
        assert!(!mv.is_capture());
        assert!(!mv.is_promotion());
        assert!(all.contains(mv));
    }

    assert_eq!(captures.len() + quiets.len(), all.len());
}

#[test]
fn generated_moves_match_legal_oracle_on_selected_positions() {
    let positions = [
        Position::startpos(),
        Position::from_fen("4r2k/8/8/8/8/8/4B3/4K3 w - - 0 1").unwrap(),
        Position::from_fen("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1").unwrap(),
        Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap(),
        Position::from_fen("4k3/8/8/8/8/5n2/4r3/4K3 w - - 0 1").unwrap(),
    ];

    for pos in &positions {
        assert_eq!(generated_moves(pos), legal_oracle(pos));
    }
}

#[test]
fn generated_moves_round_trip_position_state() {
    let mut positions = [
        Position::startpos(),
        KIWIPETE.position(),
        Position::from_fen("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1").unwrap(),
    ];

    for pos in &mut positions {
        let original = pos.clone();
        let moves = generated_moves(pos);

        for mv in moves {
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

#[test]
fn startpos_perft_matches_known_counts() {
    let mut pos = STARTPOS.position();
    assert_eq!(perft(&mut pos, 1), 20);
    assert_eq!(perft(&mut pos, 2), 400);
    assert_eq!(perft(&mut pos, 3), 8_902);
    assert_eq!(perft(&mut pos, 4), 197_281);
    assert_eq!(perft(&mut pos, 5), 4_865_609);
}

#[test]
fn reference_perft_positions_match_deeper_counts() {
    let mut kiwipete = KIWIPETE.position();
    assert_eq!(perft(&mut kiwipete, 1), 48);
    assert_eq!(perft(&mut kiwipete, 2), 2_039);
    assert_eq!(perft(&mut kiwipete, 3), 97_862);
    assert_eq!(perft(&mut kiwipete, 4), 4_085_603);

    let mut pos3 = POSITION_3.position();
    assert_eq!(perft(&mut pos3, 1), 14);
    assert_eq!(perft(&mut pos3, 2), 191);
    assert_eq!(perft(&mut pos3, 3), 2_812);
    assert_eq!(perft(&mut pos3, 4), 43_238);
    assert_eq!(perft(&mut pos3, 5), 674_624);

    let mut pos4 = POSITION_4.position();
    assert_eq!(perft(&mut pos4, 1), 6);
    assert_eq!(perft(&mut pos4, 2), 264);
    assert_eq!(perft(&mut pos4, 3), 9_467);
    assert_eq!(perft(&mut pos4, 4), 422_333);

    let mut pos5 = POSITION_5.position();
    assert_eq!(perft(&mut pos5, 1), 44);
    assert_eq!(perft(&mut pos5, 2), 1_486);
    assert_eq!(perft(&mut pos5, 3), 62_379);
    assert_eq!(perft(&mut pos5, 4), 2_103_487);

    let mut pos6 = POSITION_6.position();
    assert_eq!(perft(&mut pos6, 1), 46);
    assert_eq!(perft(&mut pos6, 2), 2_079);
    assert_eq!(perft(&mut pos6, 3), 89_890);
    assert_eq!(perft(&mut pos6, 4), 3_894_594);
}

#[test]
#[ignore = "deep perft reference"]
fn deep_reference_startpos_depth_6() {
    let mut pos = STARTPOS.position();
    assert_eq!(perft(&mut pos, 6), 119_060_324);
}

#[test]
#[ignore = "deep perft reference"]
fn deep_reference_kiwipete_depth_5() {
    let mut pos = KIWIPETE.position();
    assert_eq!(perft(&mut pos, 5), 193_690_690);
}

#[test]
#[ignore = "deep perft reference"]
fn deep_reference_position3_depth_6() {
    let mut pos = POSITION_3.position();
    assert_eq!(perft(&mut pos, 6), 11_030_083);
}

#[test]
#[ignore = "deep perft reference"]
fn deep_reference_position4_depth_5() {
    let mut pos = POSITION_4.position();
    assert_eq!(perft(&mut pos, 5), 15_833_292);
}

#[test]
#[ignore = "deep perft reference"]
fn deep_reference_position5_depth_5() {
    let mut pos = POSITION_5.position();
    assert_eq!(perft(&mut pos, 5), 89_941_194);
}

#[test]
#[ignore = "deep perft reference"]
fn deep_reference_position6_depth_5() {
    let mut pos = POSITION_6.position();
    assert_eq!(perft(&mut pos, 5), 164_075_551);
}

#[test]
fn castling_and_en_passant_moves_appear_when_legal() {
    let castle = Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    let mut list = MoveList::new();
    generate_all(&castle, &mut list);
    assert!(list.contains(Move::new(sq("e1"), sq("g1"), MoveKind::Castle)));
    assert!(list.contains(Move::new(sq("e1"), sq("c1"), MoveKind::Castle)));

    let ep = Position::from_fen("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1").unwrap();
    list.clear();
    generate_all(&ep, &mut list);
    assert!(list.contains(Move::new(sq("e5"), sq("d6"), MoveKind::EnPassant)));
}
