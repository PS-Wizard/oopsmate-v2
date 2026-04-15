use std::env;

use oopsmate_core::Position;
use oopsmate_movegen::{
    KIWIPETE, MoveList, POSITION_3, POSITION_4, POSITION_5, POSITION_6, PerftCase, STARTPOS,
    generate_all, perft,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (mut pos, depth) = match args.as_slice() {
        [name, depth] => (
            lookup_case(name)
                .map(PerftCase::position)
                .unwrap_or_else(|| Position::from_fen(name).expect("unknown case and invalid FEN")),
            depth.parse::<u32>().expect("depth must be an integer"),
        ),
        _ => {
            eprintln!(
                "usage: cargo run -p oopsmate-movegen --release --bin perft_divide -- <case|fen> <depth>"
            );
            std::process::exit(2);
        }
    };

    let mut moves = MoveList::new();
    generate_all(&pos, &mut moves);

    let mut total = 0u64;
    let mut entries: Vec<(u16, u64)> = Vec::with_capacity(moves.len());
    for &mv in moves.as_slice() {
        pos.make_move(mv);
        let nodes = perft(&mut pos, depth - 1);
        pos.unmake_move(mv);
        total += nodes;
        entries.push((mv.0, nodes));
    }

    entries.sort_by_key(|(mv, _)| *mv);
    for (raw, nodes) in entries {
        let mv = oopsmate_core::Move(raw);
        println!("{}: {}", to_uci(mv), nodes);
    }
    println!("total: {total}");
}

fn to_uci(mv: oopsmate_core::Move) -> String {
    let from = mv.from();
    let to = mv.to();
    let mut text = String::with_capacity(5);
    text.push((b'a' + from.file()) as char);
    text.push((b'1' + from.rank()) as char);
    text.push((b'a' + to.file()) as char);
    text.push((b'1' + to.rank()) as char);
    if let Some(piece) = mv.kind().promotion_piece() {
        text.push(match piece {
            oopsmate_core::Piece::Knight => 'n',
            oopsmate_core::Piece::Bishop => 'b',
            oopsmate_core::Piece::Rook => 'r',
            oopsmate_core::Piece::Queen => 'q',
            _ => unreachable!(),
        });
    }
    text
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
