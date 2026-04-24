use std::sync::atomic::AtomicBool;

use oopsmate_core::{Move, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::SearchMemory;
use oopsmate_movegen::{MoveList, analyze, generate_all_with_analysis};

use crate::alphabeta::search_node;
use crate::control::{SearchContext, SearchInterrupted};
use crate::limits::SearchLimits;
use crate::picker::{MovePicker, TtMode};
use crate::types::{SearchResult, mate_score};

const MAX_SEARCH_DEPTH: u8 = 64;

pub fn search<E: Evaluator>(
    position: &Position,
    limits: SearchLimits,
    stop: &AtomicBool,
    memory: &mut SearchMemory,
    evaluator: &E,
) -> SearchResult {
    search_with_reporter(position, limits, stop, memory, evaluator, |_| {})
}

pub fn search_with_reporter<E: Evaluator, F: FnMut(&SearchResult)>(
    position: &Position,
    limits: SearchLimits,
    stop: &AtomicBool,
    memory: &mut SearchMemory,
    evaluator: &E,
    mut report: F,
) -> SearchResult {
    memory.new_search();

    let max_depth = limits.depth.unwrap_or(MAX_SEARCH_DEPTH);
    let mut pos = position.clone();
    let mut ctx = SearchContext::new(stop, limits, pos.side_to_move(), &mut memory.tt);

    // Root still pre-generates once here only to detect terminal root positions and keep a
    // fallback legal move if the search is stopped before depth 1 finishes.
    let root_analysis = analyze(&pos);
    let mut root_moves = MoveList::new();
    generate_all_with_analysis(&pos, &root_analysis, &mut root_moves);

    // No moves -> current side in check -> checkmate -> loosing mate score
    // No moves -> current side in NOT in check -> stalemate
    if root_moves.is_empty() {
        return SearchResult {
            best_move: None,
            score: if root_analysis.in_check() {
                -mate_score(0)
            } else {
                0
            },
            depth: 0,
            nodes: 0,
            time_ms: ctx.elapsed_ms(),
        };
    }

    // Fallback move, just to have some legal move incase the search gets stopped before completing
    // depth 1.
    let fallback_move = root_moves.as_slice()[0];
    let mut best = SearchResult {
        best_move: Some(fallback_move),
        score: evaluator.evaluate(&pos),
        depth: 0,
        nodes: 0,
        time_ms: 0,
    };

    if max_depth == 0 {
        best.nodes = ctx.nodes();
        best.time_ms = ctx.elapsed_ms();
        return best;
    }

    // Iterative Deepening Loop
    for depth in 1..=max_depth {
        match search_root(&mut pos, depth, &mut ctx, evaluator) {
            Ok((best_move, score)) => {
                best.best_move = Some(best_move);
                best.score = score;
                best.depth = depth;
                best.nodes = ctx.nodes();
                best.time_ms = ctx.elapsed_ms();
                report(&best);
            }
            Err(_) => break,
        }

        // Search deadline only stops between iterations, not during them.
        // Soft deadline can be exceeded a lil, but hard deadline is enforced deeper in the tree.
        if ctx.reached_soft_deadline() || ctx.should_stop_now() {
            break;
        }
    }

    best.nodes = ctx.nodes();
    best.time_ms = ctx.elapsed_ms();
    best
}

fn search_root<E: Evaluator>(
    pos: &mut Position,
    depth: u8,
    ctx: &mut SearchContext<'_>,
    evaluator: &E,
) -> Result<(Move, i32), SearchInterrupted> {
    let analysis = analyze(pos);
    let tt_move = ctx
        .tt
        .probe(pos.hash(), 0)
        .map_or(Move::NULL, |hit| hit.best_move);
    let mut picker = MovePicker::new(pos, &analysis, tt_move, TtMode::ValidateInStage);
    let mut best_move = Move::NULL;
    let mut alpha = i32::MIN / 2;
    let beta = i32::MAX / 2;
    let mut saw_move = false;

    while let Some(mv) = picker.next_move(pos, &analysis) {
        saw_move = true;
        if ctx.should_stop_now() {
            return Err(SearchInterrupted);
        }

        pos.make_move(mv);
        let score = match search_node(pos, depth - 1, 1, -beta, -alpha, ctx, evaluator) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(mv);
                return Err(err);
            }
        };
        pos.unmake_move(mv);

        if score > alpha {
            alpha = score;
            best_move = mv;
        }
    }

    debug_assert!(saw_move, "root search called without legal moves");
    Ok((best_move, alpha))
}
