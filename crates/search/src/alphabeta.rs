use oopsmate_core::{Move, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::analyze;

use crate::control::{SearchContext, SearchInterrupted};
use crate::picker::{MovePicker, TtMode};
use crate::qsearch::{NO_STATIC_EVAL, qsearch};
use crate::types::mate_score;

pub(crate) fn search_node<E: Evaluator>(
    pos: &mut Position,
    depth: u8,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    evaluator: &E,
) -> Result<i32, SearchInterrupted> {
    if depth == 0 {
        return qsearch(pos, ply, alpha, beta, ctx, evaluator);
    }

    ctx.enter_node()?;

    if pos.rule50() >= 100 || pos.is_repetition() {
        return Ok(0);
    }

    let hash = pos.hash();
    let alpha_orig = alpha;
    let mut tt_move = Move::NULL;

    if let Some(hit) = ctx.tt.probe(hash, ply) {
        tt_move = hit.best_move;
        if hit.depth >= depth {
            match hit.bound {
                Bound::Exact => return Ok(hit.score),
                Bound::Lower if hit.score >= beta => return Ok(hit.score),
                Bound::Upper if hit.score <= alpha => return Ok(hit.score),
                _ => {}
            }
        }
    }

    let analysis = analyze(pos);
    let tt_mode = if analysis.in_check() {
        TtMode::ValidateInStage
    } else {
        TtMode::BlindTrust
    };
    let mut picker = MovePicker::new(pos, &analysis, tt_move, tt_mode);
    let mut best_move = Move::NULL;
    let mut best_score = i32::MIN / 2;
    let mut saw_legal_move = false;

    while let Some(mv) = picker.next_move(pos, &analysis) {
        saw_legal_move = true;
        pos.make_move(mv);
        let score = match search_node(pos, depth - 1, ply + 1, -beta, -alpha, ctx, evaluator) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(mv);
                return Err(err);
            }
        };
        pos.unmake_move(mv);

        if score > best_score {
            best_score = score;
            best_move = mv;
        }

        if score >= beta {
            ctx.tt
                .store(hash, ply, mv, score, NO_STATIC_EVAL, depth, Bound::Lower);
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
    }

    if !saw_legal_move {
        let score = if analysis.in_check() {
            -mate_score(ply)
        } else {
            0
        };
        ctx.tt.store(
            hash,
            ply,
            Move::NULL,
            score,
            NO_STATIC_EVAL,
            depth,
            Bound::Exact,
        );
        return Ok(score);
    }

    let bound = if best_score <= alpha_orig {
        Bound::Upper
    } else {
        Bound::Exact
    };
    ctx.tt.store(
        hash,
        ply,
        best_move,
        best_score,
        NO_STATIC_EVAL,
        depth,
        bound,
    );

    Ok(best_score)
}
