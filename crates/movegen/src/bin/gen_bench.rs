use std::env;
use std::hint::black_box;
use std::time::Instant;

use oopsmate_movegen::{
    KIWIPETE, MoveList, POSITION_3, POSITION_4, POSITION_5, POSITION_6, PerftCase, STARTPOS,
    generate_all,
};

const DEFAULT_ITERS: u64 = 1_000_000;
const SUITE: &[&PerftCase] = &[
    &STARTPOS,
    &KIWIPETE,
    &POSITION_3,
    &POSITION_4,
    &POSITION_5,
    &POSITION_6,
];

fn main() {
    let iterations = match env::args().nth(1) {
        Some(arg) => arg.parse::<u64>().expect("iterations must be an integer"),
        None => DEFAULT_ITERS,
    };

    println!("root legal movegen benchmark");
    println!("iterations per case: {iterations}");
    println!();

    let mut total_moves = 0u64;
    let mut total_checksum = 0u64;
    let start = Instant::now();

    for &case in SUITE {
        let (moves, checksum) = run_case(case, iterations);
        total_moves += moves;
        total_checksum = total_checksum.wrapping_add(checksum);
    }

    let elapsed = start.elapsed();
    let mps = total_moves as f64 / elapsed.as_secs_f64();

    println!();
    println!(
        "total  moves={}  time={:.3}s  mps={:.0}  checksum={}",
        total_moves,
        elapsed.as_secs_f64(),
        mps,
        total_checksum
    );
}

fn run_case(case: &PerftCase, iterations: u64) -> (u64, u64) {
    let pos = case.position();
    let mut list = MoveList::new();
    let mut generated = 0u64;
    let mut checksum = 0u64;

    let start = Instant::now();
    for _ in 0..iterations {
        generate_all(black_box(&pos), black_box(&mut list));
        generated += black_box(list.len() as u64);
        checksum = checksum.wrapping_add(black_box(checksum_moves(&list)));
    }
    let elapsed = start.elapsed();
    let mps = generated as f64 / elapsed.as_secs_f64();

    println!(
        "{:<10} moves={}  time={:.3}s  mps={:.0}  checksum={}",
        case.name,
        generated,
        elapsed.as_secs_f64(),
        mps,
        checksum
    );

    (generated, checksum)
}

#[inline(always)]
fn checksum_moves(list: &MoveList) -> u64 {
    let mut acc = 0u64;
    for &mv in list.as_slice() {
        acc = acc.rotate_left(5) ^ mv.0 as u64;
    }
    acc
}
