use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use oopsmate_core::{Move, Piece, Position};
use oopsmate_eval::NnueEval;
use oopsmate_memory::SearchMemory;
use oopsmate_movegen::{generate_all, MoveList};
use oopsmate_search::{mate_in, search_with_reporter, ClockLimits, SearchLimits, SearchResult};

const ENGINE_AUTHOR: &str = "Swoyam P.";
const DEFAULT_TT_MIB: usize = 64;

pub fn run() {
    let stdin = io::stdin();
    let mut position = Position::startpos();
    let mut state = Some(EngineState::new(DEFAULT_TT_MIB));
    let mut worker = None;

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "uci" => {
                print_line(format!("id name {}", oopsmate_core::engine_name()));
                print_line(format!("id author {ENGINE_AUTHOR}"));
                print_line("uciok");
            }
            "isready" => print_line("readyok"),
            "ucinewgame" => {
                stop_and_join(&mut worker, &mut state);
                state.as_mut().expect("engine state missing").memory.clear();
                position = Position::startpos();
            }
            "position" => {
                stop_and_join(&mut worker, &mut state);
                if let Err(err) = set_position(&mut position, &tokens[1..]) {
                    eprintln!("position error: {err}");
                }
            }
            "go" => {
                stop_and_join(&mut worker, &mut state);
                worker = Some(spawn_search(
                    position.clone(),
                    parse_go(&tokens[1..]),
                    state.take().expect("engine state missing"),
                ));
            }
            "stop" => stop_and_join(&mut worker, &mut state),
            "quit" => {
                stop_and_join(&mut worker, &mut state);
                break;
            }
            "setoption" | "register" | "ponderhit" | "debug" => {}
            _ => {}
        }
    }

    stop_and_join(&mut worker, &mut state);
}

struct EngineState {
    memory: SearchMemory,
    evaluator: NnueEval,
}

impl EngineState {
    fn new(tt_mib: usize) -> Self {
        Self {
            memory: SearchMemory::new(tt_mib),
            evaluator: NnueEval::load_default().expect("load NNUE networks"),
        }
    }
}

struct SearchWorker {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<EngineState>,
}

fn spawn_search(
    position: Position,
    limits: SearchLimits,
    mut state: EngineState,
) -> SearchWorker {
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let handle = thread::Builder::new()
        .name("oopsmate-search".into())
        .spawn(move || {
            let result = search_with_reporter(
                &position,
                limits,
                worker_stop.as_ref(),
                &mut state.memory,
                &mut state.evaluator,
                print_search_info,
            );
            print_search_result(result);
            state
        })
        .expect("failed to spawn search worker");

    SearchWorker { stop, handle }
}

fn stop_and_join(worker: &mut Option<SearchWorker>, state: &mut Option<EngineState>) {
    if let Some(worker) = worker.take() {
        worker.stop.store(true, Ordering::Relaxed);
        *state = Some(worker.handle.join().expect("search worker panicked"));
    }
}

fn print_search_result(result: SearchResult) {
    if result.depth == 0 {
        print_search_info(&result);
    }

    let bestmove = result
        .best_move
        .map_or_else(|| "0000".to_owned(), move_to_uci);
    print_line(format!("bestmove {bestmove}"));
}

fn print_search_info(result: &SearchResult) {
    let nps = if result.time_ms > 0 {
        result.nodes.saturating_mul(1000) / result.time_ms
    } else {
        0
    };
    let pv = result
        .best_move
        .map(move_to_uci)
        .unwrap_or_else(|| "0000".into());

    if let Some(mate) = mate_in(result.score) {
        print_line(format!(
            "info depth {} score mate {} nodes {} time {} nps {} pv {}",
            result.depth, mate, result.nodes, result.time_ms, nps, pv
        ));
    } else {
        print_line(format!(
            "info depth {} score cp {} nodes {} time {} nps {} pv {}",
            result.depth, result.score, result.nodes, result.time_ms, nps, pv
        ));
    }
}

fn parse_go(tokens: &[&str]) -> SearchLimits {
    let mut limits = SearchLimits::new();
    let mut clock = ClockLimits::default();
    let mut has_clock = false;
    let mut infinite = false;
    let mut index = 0;

    while index < tokens.len() {
        match tokens[index] {
            "depth" => {
                index += 1;
                if index < tokens.len() {
                    limits.depth = tokens[index].parse().ok();
                }
            }
            "movetime" => {
                index += 1;
                if index < tokens.len() {
                    limits.movetime_ms = tokens[index].parse().ok();
                }
            }
            "wtime" => {
                index += 1;
                if index < tokens.len() {
                    has_clock = true;
                    clock.white_time_ms = tokens[index].parse().unwrap_or(0);
                }
            }
            "btime" => {
                index += 1;
                if index < tokens.len() {
                    has_clock = true;
                    clock.black_time_ms = tokens[index].parse().unwrap_or(0);
                }
            }
            "winc" => {
                index += 1;
                if index < tokens.len() {
                    has_clock = true;
                    clock.white_increment_ms = tokens[index].parse().unwrap_or(0);
                }
            }
            "binc" => {
                index += 1;
                if index < tokens.len() {
                    has_clock = true;
                    clock.black_increment_ms = tokens[index].parse().unwrap_or(0);
                }
            }
            "movestogo" => {
                index += 1;
                if index < tokens.len() {
                    has_clock = true;
                    clock.movestogo = tokens[index].parse().ok();
                }
            }
            "infinite" => infinite = true,
            _ => {}
        }
        index += 1;
    }

    if has_clock {
        limits.clock = Some(clock);
    }

    if limits.depth.is_none() && limits.movetime_ms.is_none() && !has_clock && !infinite {
        limits.depth = Some(4);
    }

    limits
}

fn set_position(position: &mut Position, tokens: &[&str]) -> Result<(), String> {
    if tokens.is_empty() {
        return Err("missing position arguments".into());
    }

    let (next, index) = match tokens[0] {
        "startpos" => (Position::startpos(), 1),
        "fen" => {
            let moves_index = tokens
                .iter()
                .position(|token| *token == "moves")
                .unwrap_or(tokens.len());
            let fen = tokens[1..moves_index].join(" ");
            (
                Position::from_fen(&fen).map_err(|err| err.to_string())?,
                moves_index,
            )
        }
        _ => return Err("expected 'startpos' or 'fen'".into()),
    };
    *position = next;

    if index < tokens.len() {
        if tokens[index] != "moves" {
            return Err("expected 'moves' after position".into());
        }

        for mv in &tokens[index + 1..] {
            apply_uci_move(position, mv)?;
        }
    }

    Ok(())
}

fn apply_uci_move(position: &mut Position, text: &str) -> Result<(), String> {
    let mut moves = MoveList::new();
    generate_all(position, &mut moves);

    let mv = moves
        .as_slice()
        .iter()
        .copied()
        .find(|&mv| move_to_uci(mv) == text)
        .ok_or_else(|| format!("illegal move: {text}"))?;

    position.make_move(mv);
    Ok(())
}

pub(crate) fn move_to_uci(mv: Move) -> String {
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

fn print_line(text: impl AsRef<str>) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{}", text.as_ref());
    let _ = stdout.flush();
}
