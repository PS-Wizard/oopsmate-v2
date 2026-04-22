use nnuebie_v2::{NnueContext, NnueNetworks};
use oopsmate_core::{Move, MoveKind, Position, Square};
use std::hint::black_box;
use std::time::Instant;

fn sq(file: u8, rank: u8) -> Square {
    Square::from_file_rank(file - b'a', rank - 1).expect("valid square")
}

fn report_section(label: &str, evals: usize, start: Instant) -> f64 {
    let duration = start.elapsed();
    let nps = evals as f64 / duration.as_secs_f64();
    println!(
        "{}: {:.2} evals/sec ({} evals, {:.2}s)",
        label,
        nps,
        evals,
        duration.as_secs_f64()
    );
    nps
}

fn main() {
    println!("Loading networks...");
    let networks = NnueNetworks::load_default().expect("load default networks");

    let fen_list = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4",
        "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1",
        "r1bqkb1r/pppp1ppp/2n2n2/3Pp3/4P3/2N2N2/PPP2PPP/R1BQKB1R b KQkq - 0 1",
        "r1bq1rk1/ppp1npbp/2np2p1/4p3/2P4N/2NP2P1/PP2PPBP/R1BQ1RK1 w - - 0 1",
        "r1bq1rk1/1pp2pbN/2np4/4p3/7N/3P2P1/1P2PPBP/R1BQ1RK1 w - - 0 1",
        "4k3/8/8/8/8/8/4K3/8 w - - 0 1",
        "4k3/8/8/8/8/8/4K2P/8 w - - 0 1",
        "4k3/8/8/8/8/8/4K2R/8 w - - 0 1",
        "6k1/5ppp/8/8/8/8/5PPP/6K1 w - - 0 1",
        "4k3/8/8/8/8/3p4/4K3/8 b - - 0 1",
        "6k1/8/8/8/8/8/4K3/6R1 w - - 0 1",
    ];

    let positions: Vec<Position> = fen_list
        .iter()
        .map(|fen| Position::from_fen(fen).expect("valid benchmark FEN"))
        .collect();

    let mut warm_ctx = NnueContext::new();
    if let Some(first) = positions.first() {
        networks.reset_context(first, &mut warm_ctx);
        for _ in 0..100 {
            black_box(networks.evaluate(first, &mut warm_ctx));
        }
    }

    println!("Benchmarking with {} FENs...", positions.len());

    let mut results: Vec<(&'static str, f64)> = Vec::new();

    // Section 1: Full refresh (set_position/reset_context) across a small FEN corpus
    let full_refresh_target = 2_000_000usize;
    let fen_count = positions.len().max(1);
    let full_refresh_loops = (full_refresh_target / fen_count).max(1);
    let full_refresh_evals = full_refresh_loops * fen_count;
    let mut full_ctx = NnueContext::new();

    let start = Instant::now();
    for _ in 0..full_refresh_loops {
        for position in &positions {
            networks.reset_context(position, &mut full_ctx);
            black_box(networks.evaluate(position, &mut full_ctx));
        }
    }
    let nps = report_section("Full Refresh (FEN corpus)", full_refresh_evals, start);
    results.push(("Full Refresh (FEN corpus)", nps));

    // Section 2: Incremental mixed moves on a midgame-like position
    let base_fen = "r1bqkbnr/1ppp1ppp/p1n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 4";
    let mut position = Position::from_fen(base_fen).expect("valid base FEN");
    let mut incremental_ctx = NnueContext::new();
    networks.reset_context(&position, &mut incremental_ctx);

    let toggles = [
        Move::new(sq(b'a', 2), sq(b'a', 3), MoveKind::Quiet),
        Move::new(sq(b'f', 3), sq(b'g', 5), MoveKind::Quiet),
        Move::new(sq(b'b', 5), sq(b'c', 6), MoveKind::Capture),
        Move::new(sq(b'e', 1), sq(b'f', 1), MoveKind::Quiet),
        Move::new(sq(b'a', 6), sq(b'a', 5), MoveKind::Quiet),
        Move::new(sq(b'b', 7), sq(b'b', 6), MoveKind::Quiet),
    ];

    let toggle_cycles = 1_000_000usize;
    let incremental_evals = toggle_cycles * toggles.len() * 2;

    let start = Instant::now();
    for _ in 0..toggle_cycles {
        for &mv in &toggles {
            incremental_ctx.push_move(&position, mv);
            position.make_move(mv);
            black_box(networks.evaluate(&position, &mut incremental_ctx));
            position.unmake_move(mv);
            incremental_ctx.pop();
            black_box(networks.evaluate(&position, &mut incremental_ctx));
        }
    }
    let nps = report_section("Incremental (mixed moves)", incremental_evals, start);
    results.push(("Incremental (mixed moves)", nps));

    // Section 3: King-move refresh cost (forces one-perspective refresh)
    networks.reset_context(&position, &mut incremental_ctx);
    let king_cycles = 2_500_000usize;
    let king_evals = king_cycles * 2;
    let king_move = Move::new(sq(b'e', 1), sq(b'f', 1), MoveKind::Quiet);

    let start = Instant::now();
    for _ in 0..king_cycles {
        incremental_ctx.push_move(&position, king_move);
        position.make_move(king_move);
        black_box(networks.evaluate(&position, &mut incremental_ctx));
        position.unmake_move(king_move);
        incremental_ctx.pop();
        black_box(networks.evaluate(&position, &mut incremental_ctx));
    }
    let nps = report_section("King Refresh (toggle)", king_evals, start);
    results.push(("King Refresh (toggle)", nps));

    println!("\nSummary:");
    for (label, nps) in results {
        println!("- {}: {:.2} evals/sec", label, nps);
    }
}
