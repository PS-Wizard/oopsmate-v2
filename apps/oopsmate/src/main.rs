mod uci;

use oopsmate_core::Position;
use oopsmate_movegen::{MoveList, generate_all};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.split_first() {
        None => uci::run(),
        Some((command, fen_parts)) if command == "moves" => {
            let pos = if fen_parts.is_empty() {
                Position::startpos()
            } else {
                let fen = fen_parts.join(" ");
                Position::from_fen(&fen).expect("invalid FEN")
            };
            print_moves(&pos);
        }
        _ => {
            eprintln!("usage:");
            eprintln!("  cargo run -p oopsmate-v2");
            eprintln!("  cargo run -p oopsmate-v2 -- moves");
            eprintln!("  cargo run -p oopsmate-v2 -- moves '<fen>'");
            std::process::exit(2);
        }
    }
}

fn print_moves(pos: &Position) {
    let mut moves = MoveList::new();
    generate_all(pos, &mut moves);

    println!("legal moves: {}", moves.len());
    for &mv in moves.as_slice() {
        println!("{}", uci::move_to_uci(mv));
    }
}
