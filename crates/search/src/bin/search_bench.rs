use std::sync::atomic::AtomicBool;
use std::time::Instant;

use oopsmate_core::{Move, MoveKind, Piece, Position};
use oopsmate_eval::NnueEval;
use oopsmate_memory::SearchMemory;
use oopsmate_movegen::PERFT_CASES;
#[cfg(feature = "telemetry")]
use oopsmate_search::SearchTelemetry;
use oopsmate_search::{search, SearchLimits};

const DEFAULT_DEPTH: u8 = 5;
const DEFAULT_RUNS: usize = 3;
const DEFAULT_TT_MIB: usize = 64;

fn main() {
    let mut args = std::env::args().skip(1);
    let depth = args
        .next()
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(DEFAULT_DEPTH);
    let runs = args
        .next()
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(DEFAULT_RUNS);
    let tt_mib = args
        .next()
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(DEFAULT_TT_MIB);

    let stop = AtomicBool::new(false);
    let mut evaluator = NnueEval::load_default().expect("load NNUE networks");
    let mut total_nodes = 0u64;
    let mut total_nanos = 0u128;
    #[cfg(feature = "telemetry")]
    let mut total_telemetry = SearchTelemetry::default();

    println!("search benchmark depth={depth} runs={runs} tt={tt_mib}MiB");
    println!(
        "{:<10} {:>5} {:>12} {:>10} {:>12} {:>8} {:>8}",
        "case", "run", "nodes", "time_ms", "nps", "score", "best"
    );

    for case in PERFT_CASES {
        let position = Position::from_fen(case.fen).expect("valid benchmark FEN");

        for run in 1..=runs {
            let mut memory = SearchMemory::new(tt_mib);
            let start = Instant::now();
            let result = search(
                &position,
                SearchLimits::depth(depth),
                &stop,
                &mut memory,
                &mut evaluator,
            );
            let elapsed = start.elapsed();
            let nanos = elapsed.as_nanos().max(1);
            let nps = (result.nodes as u128 * 1_000_000_000 / nanos) as u64;

            total_nodes += result.nodes;
            total_nanos += nanos;
            #[cfg(feature = "telemetry")]
            total_telemetry.add(result.telemetry);

            println!(
                "{:<10} {:>5} {:>12} {:>10.3} {:>12} {:>8} {:>8}",
                case.name,
                run,
                result.nodes,
                elapsed.as_secs_f64() * 1000.0,
                nps,
                result.score,
                result
                    .best_move
                    .map_or_else(|| "0000".to_owned(), move_to_uci),
            );
        }
    }

    let total_nps = (total_nodes as u128 * 1_000_000_000 / total_nanos.max(1)) as u64;
    println!(
        "total nodes={} time_ms={:.3} nps={}",
        total_nodes,
        total_nanos as f64 / 1_000_000.0,
        total_nps
    );

    #[cfg(feature = "telemetry")]
    print_telemetry(total_telemetry);
}

#[cfg(feature = "telemetry")]
fn print_telemetry(t: SearchTelemetry) {
    println!("telemetry:");
    println!(
        "  main_nodes={} q_nodes={} eval_calls={}",
        t.main_nodes, t.q_nodes, t.eval_calls
    );
    println!(
        "  tt_hits={} tt_cutoffs={} tt_static_eval_reuses={}",
        t.tt_hits, t.tt_cutoffs, t.tt_static_eval_reuses
    );
    println!(
        "  razor_cutoffs={} rfp_cutoffs={} futility_skips={} late_quiet_skips={}",
        t.razor_cutoffs, t.rfp_cutoffs, t.futility_skips, t.late_quiet_skips
    );
    println!(
        "  null_attempts={} null_cutoffs={} probcut_attempts={} probcut_qsearch_passes={} probcut_cutoffs={}",
        t.null_attempts,
        t.null_cutoffs,
        t.probcut_attempts,
        t.probcut_qsearch_passes,
        t.probcut_cutoffs
    );
    println!(
        "  lmr_attempts={} lmr_cutoffs={} lmr_researches={}",
        t.lmr_attempts, t.lmr_cutoffs, t.lmr_researches
    );
}

fn move_to_uci(mv: Move) -> String {
    let from = mv.from();
    let to = mv.to();
    let mut text = String::with_capacity(5);
    text.push((b'a' + from.file()) as char);
    text.push((b'1' + from.rank()) as char);
    text.push((b'a' + to.file()) as char);
    text.push((b'1' + to.rank()) as char);

    let kind = (mv.0 >> 12) as u8;
    if matches!(kind, 8..=15) {
        text.push(
            match mv.kind().promotion_piece().expect("promotion piece") {
                Piece::Knight => 'n',
                Piece::Bishop => 'b',
                Piece::Rook => 'r',
                Piece::Queen => 'q',
                Piece::Pawn | Piece::King => unreachable!(),
            },
        );
    } else {
        debug_assert!(!matches!(mv.kind(), MoveKind::PromotionKnight));
    }

    text
}
