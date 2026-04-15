use std::env;
use std::time::Instant;

use oopsmate_movegen::{
    KIWIPETE, POSITION_3, POSITION_4, POSITION_5, POSITION_6, PerftCase, STARTPOS, perft,
};

const DEFAULT_SUITE: &[(&PerftCase, u32)] = &[
    (&STARTPOS, 5),
    (&KIWIPETE, 4),
    (&POSITION_3, 5),
    (&POSITION_4, 4),
    (&POSITION_5, 4),
    (&POSITION_6, 4),
];

const DEEP_SUITE: &[(&PerftCase, u32)] = &[
    (&STARTPOS, 6),
    (&KIWIPETE, 5),
    (&POSITION_3, 6),
    (&POSITION_4, 5),
    (&POSITION_5, 5),
    (&POSITION_6, 5),
];

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.as_slice() {
        [] => run_suite("default", DEFAULT_SUITE),
        [flag] if flag == "--deep" => run_suite("deep", DEEP_SUITE),
        [name, depth] => {
            let depth: u32 = depth.parse().expect("depth must be an integer");
            let case = lookup_case(name).expect("unknown perft case");
            run_case(case, depth);
        }
        _ => {
            eprintln!("usage:");
            eprintln!("  cargo run -p oopsmate-movegen --release --bin perft_bench");
            eprintln!("  cargo run -p oopsmate-movegen --release --bin perft_bench -- --deep");
            eprintln!(
                "  cargo run -p oopsmate-movegen --release --bin perft_bench -- <case> <depth>"
            );
            eprintln!("cases: startpos, kiwipete, position3, position4, position5, position6");
            std::process::exit(2);
        }
    }
}

fn run_suite(name: &str, suite: &[(&PerftCase, u32)]) {
    println!("perft throughput benchmark ({name} suite)");
    println!();

    let mut total_nodes = 0u64;
    let start = Instant::now();
    for &(case, depth) in suite {
        total_nodes += run_case(case, depth);
    }
    let elapsed = start.elapsed();
    let nps = total_nodes as f64 / elapsed.as_secs_f64();

    println!();
    println!(
        "total  nodes={}  time={:.3}s  nps={:.0}",
        total_nodes,
        elapsed.as_secs_f64(),
        nps
    );
}

fn run_case(case: &PerftCase, depth: u32) -> u64 {
    let mut pos = case.position();
    let start = Instant::now();
    let nodes = perft(&mut pos, depth);
    let elapsed = start.elapsed();
    let nps = nodes as f64 / elapsed.as_secs_f64();
    let expected = case.nodes_at_depth(depth as usize);
    let status = match expected {
        Some(value) if value == nodes => "ok",
        Some(value) => {
            eprintln!(
                "{} d{} mismatch: expected {}, got {}",
                case.name, depth, value, nodes
            );
            "mismatch"
        }
        None => "unverified",
    };

    println!(
        "{:<10} d{}  nodes={}  time={:.3}s  nps={:.0}  {}",
        case.name,
        depth,
        nodes,
        elapsed.as_secs_f64(),
        nps,
        status
    );

    nodes
}

fn lookup_case(name: &str) -> Option<&'static PerftCase> {
    match name {
        "startpos" => Some(&STARTPOS),
        "kiwipete" => Some(&KIWIPETE),
        "position3" => Some(&POSITION_3),
        "position4" => Some(&POSITION_4),
        "position5" => Some(&POSITION_5),
        "position6" => Some(&POSITION_6),
        _ => None,
    }
}
