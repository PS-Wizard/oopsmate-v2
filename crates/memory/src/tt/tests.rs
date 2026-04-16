use std::mem::{align_of, size_of};

use oopsmate_core::{Move, MoveKind, Square};

use super::entry::{Bound, TtEntry};
use super::table::Bucket;
use super::{MATE_SCORE, TranspositionTable, mate_in};

fn make_move(from: &str, to: &str) -> Move {
    Move::new(
        Square::from_algebraic(from).unwrap(),
        Square::from_algebraic(to).unwrap(),
        MoveKind::Quiet,
    )
}

#[test]
fn tt_entry_layout_is_16_bytes_and_16_aligned() {
    assert_eq!(size_of::<TtEntry>(), 16);
    assert_eq!(align_of::<TtEntry>(), 16);
}

#[test]
fn bucket_layout_is_64_bytes_and_64_aligned() {
    assert_eq!(size_of::<Bucket>(), 64);
    assert_eq!(align_of::<Bucket>(), 64);
}

#[test]
fn sizing_math_rounds_down_to_power_of_two_buckets() {
    let tt = TranspositionTable::new(64);
    assert_eq!(tt.bucket_count(), 1_048_576);
    assert_eq!(tt.size_mib(), 64);
}

#[test]
fn empty_probe_returns_none() {
    let tt = TranspositionTable::new(0);
    assert!(tt.probe(0x1234_5678_9abc_def0, 0).is_none());
}

#[test]
fn store_and_probe_round_trip() {
    let mut tt = TranspositionTable::new(0);
    let hash = 0x1234_5678_9abc_def0;
    let mv = make_move("e2", "e4");

    tt.store(hash, 3, mv, 42, 17, 7, Bound::Exact);

    let hit = tt.probe(hash, 3).unwrap();
    assert_eq!(hit.best_move, mv);
    assert_eq!(hit.score, 42);
    assert_eq!(hit.static_eval, 17);
    assert_eq!(hit.depth, 7);
    assert_eq!(hit.bound, Bound::Exact);
}

#[test]
fn same_key_overwrites_existing_entry() {
    let mut tt = TranspositionTable::new(0);
    let hash = 0x1234_5678_9abc_def0;
    let first = make_move("e2", "e4");
    let second = make_move("d2", "d4");

    tt.store(hash, 0, first, 11, 3, 4, Bound::Lower);
    tt.store(hash, 0, second, 99, 5, 8, Bound::Upper);

    let hit = tt.probe(hash, 0).unwrap();
    assert_eq!(hit.best_move, second);
    assert_eq!(hit.score, 99);
    assert_eq!(hit.static_eval, 5);
    assert_eq!(hit.depth, 8);
    assert_eq!(hit.bound, Bound::Upper);
}

#[test]
fn depth_and_age_selects_weakest_victim() {
    let mut tt = TranspositionTable::new(0);

    let h1 = 0x0000_0001_0000_0001;
    let h2 = 0x0000_0002_0000_0002;
    let h3 = 0x0000_0003_0000_0003;
    let h4 = 0x0000_0004_0000_0004;
    let h5 = 0x0000_0005_0000_0005;

    tt.new_search();
    tt.store(h1, 0, make_move("a2", "a3"), 1, 1, 10, Bound::Exact);

    tt.new_search();
    tt.store(h2, 0, make_move("b2", "b3"), 2, 2, 5, Bound::Exact);

    tt.new_search();
    tt.store(h3, 0, make_move("c2", "c3"), 3, 3, 6, Bound::Exact);

    tt.new_search();
    tt.store(h4, 0, make_move("d2", "d3"), 4, 4, 7, Bound::Exact);

    tt.store(h5, 0, make_move("e2", "e3"), 5, 5, 1, Bound::Exact);

    assert!(tt.probe(h1, 0).is_none());
    assert!(tt.probe(h2, 0).is_some());
    assert!(tt.probe(h3, 0).is_some());
    assert!(tt.probe(h4, 0).is_some());
    assert!(tt.probe(h5, 0).is_some());
}

#[test]
fn static_eval_sentinel_is_preserved() {
    let mut tt = TranspositionTable::new(0);
    let hash = 0x1111_2222_3333_4444;

    tt.store(hash, 0, Move::NULL, 0, i16::MIN, 0, Bound::Exact);

    let hit = tt.probe(hash, 0).unwrap();
    assert_eq!(hit.static_eval, i16::MIN);
}

#[test]
fn mate_scores_round_trip_through_normalization() {
    let mut tt = TranspositionTable::new(0);
    let hash = 0x5555_6666_7777_8888;

    let winning_mate = MATE_SCORE - 7;
    let losing_mate = -MATE_SCORE + 9;

    tt.store(hash, 3, Move::NULL, winning_mate, 0, 4, Bound::Exact);
    assert_eq!(tt.probe(hash, 3).unwrap().score, winning_mate);

    tt.store(hash, 11, Move::NULL, losing_mate, 0, 4, Bound::Exact);
    assert_eq!(tt.probe(hash, 11).unwrap().score, losing_mate);
}

#[test]
fn hashfull_reflects_occupancy() {
    let mut tt = TranspositionTable::new(0);
    assert_eq!(tt.hashfull_per_mille(), 0);

    tt.store(0xaaaa_bbbb_cccc_dddd, 0, Move::NULL, 1, 1, 1, Bound::Exact);
    assert_eq!(tt.hashfull_per_mille(), 250);
}

#[test]
fn search_memory_wraps_transposition_table() {
    let mut memory = crate::SearchMemory::new(0);
    assert_eq!(memory.tt.size_mib(), 0);
    memory.new_search();
    memory.clear();
    assert!(memory.tt.probe(0xdead_beef_dead_beef, 0).is_none());
}

#[test]
fn mate_in_reports_mate_distance() {
    assert_eq!(mate_in(MATE_SCORE - 7), Some(4));
    assert_eq!(mate_in(-MATE_SCORE + 9), Some(-5));
    assert_eq!(mate_in(123), None);
}
