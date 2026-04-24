use oopsmate_core::{Move, MoveKind, Piece, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::{
    Analysis, MoveList, analyze, generate_captures_promotions_with_analysis,
    generate_evasions_with_analysis, see_ge,
};

use crate::control::{SearchContext, SearchInterrupted};
use crate::types::{is_mate_score, mate_score};

pub(crate) const NO_STATIC_EVAL: i16 = i16::MIN;

pub(crate) fn qsearch<E: Evaluator>(
    pos: &mut Position,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    evaluator: &E,
) -> Result<i32, SearchInterrupted> {
    ctx.enter_node()?;

    if pos.rule50() >= 100 || pos.is_repetition() {
        return Ok(0);
    }

    let hash = pos.hash();
    let alpha_orig = alpha;
    let mut tt_move = Move::NULL;
    let mut tt_static_eval = None;

    if let Some(hit) = ctx.tt.probe(hash, ply) {
        tt_move = hit.best_move;
        if hit.static_eval != NO_STATIC_EVAL {
            tt_static_eval = Some(i32::from(hit.static_eval));
        }

        match hit.bound {
            Bound::Exact => return Ok(hit.score),
            Bound::Lower if hit.score >= beta => return Ok(hit.score),
            Bound::Upper if hit.score <= alpha => return Ok(hit.score),
            _ => {}
        }
    }

    let analysis = analyze(pos);

    if analysis.in_check() {
        return qsearch_evasions(pos, &analysis, tt_move, ply, alpha, beta, ctx, evaluator);
    }

    let static_eval = tt_static_eval.unwrap_or_else(|| evaluator.evaluate(pos));
    let mut best_move = Move::NULL;
    let mut best_score = static_eval;

    if static_eval >= beta {
        ctx.tt.store(
            hash,
            ply,
            Move::NULL,
            static_eval,
            pack_static_eval(static_eval),
            0,
            Bound::Lower,
        );
        return Ok(static_eval);
    }

    if static_eval > alpha {
        alpha = static_eval;
    }

    let mut moves = MoveList::new();
    generate_captures_promotions_with_analysis(pos, &analysis, &mut moves);

    let mut skip = Move::NULL;
    if tt_move != Move::NULL
        && is_valid_encoded_move(tt_move)
        && is_tactical_move(tt_move)
        && moves.contains(tt_move)
    {
        skip = tt_move;
        pos.make_move(tt_move);
        let score = match qsearch(pos, ply + 1, -beta, -alpha, ctx, evaluator) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(tt_move);
                return Err(err);
            }
        };
        pos.unmake_move(tt_move);

        if score > best_score {
            best_score = score;
            best_move = tt_move;
        }

        if score >= beta {
            ctx.tt.store(
                hash,
                ply,
                tt_move,
                score,
                pack_static_eval(static_eval),
                0,
                Bound::Lower,
            );
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
    }

    let mut next = 0usize;
    while let Some(mv) = next_qmove(pos, &mut moves, &mut next, skip) {
        if delta_prune_move(pos, mv, static_eval, alpha) || see_prune_move(pos, mv) {
            continue;
        }

        pos.make_move(mv);
        let score = match qsearch(pos, ply + 1, -beta, -alpha, ctx, evaluator) {
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
            ctx.tt.store(
                hash,
                ply,
                mv,
                score,
                pack_static_eval(static_eval),
                0,
                Bound::Lower,
            );
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
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
        pack_static_eval(static_eval),
        0,
        bound,
    );

    Ok(best_score)
}

fn qsearch_evasions<E: Evaluator>(
    pos: &mut Position,
    analysis: &Analysis,
    tt_move: Move,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    evaluator: &E,
) -> Result<i32, SearchInterrupted> {
    let hash = pos.hash();
    let alpha_orig = alpha;
    let mut moves = MoveList::new();
    generate_evasions_with_analysis(pos, analysis, &mut moves);

    if moves.is_empty() {
        let score = -mate_score(ply);
        ctx.tt.store(
            hash,
            ply,
            Move::NULL,
            score,
            NO_STATIC_EVAL,
            0,
            Bound::Exact,
        );
        return Ok(score);
    }

    let mut best_move = Move::NULL;
    let mut best_score = i32::MIN / 2;
    let mut next = 0usize;
    let mut skip = Move::NULL;

    if tt_move != Move::NULL && is_valid_encoded_move(tt_move) && moves.contains(tt_move) {
        skip = tt_move;
        pos.make_move(tt_move);
        let score = match qsearch(pos, ply + 1, -beta, -alpha, ctx, evaluator) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(tt_move);
                return Err(err);
            }
        };
        pos.unmake_move(tt_move);

        best_score = score;
        best_move = tt_move;

        if score >= beta {
            ctx.tt
                .store(hash, ply, tt_move, score, NO_STATIC_EVAL, 0, Bound::Lower);
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
    }

    while let Some(mv) = next_qmove(pos, &mut moves, &mut next, skip) {
        pos.make_move(mv);
        let score = match qsearch(pos, ply + 1, -beta, -alpha, ctx, evaluator) {
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
                .store(hash, ply, mv, score, NO_STATIC_EVAL, 0, Bound::Lower);
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
    }

    let bound = if best_score <= alpha_orig {
        Bound::Upper
    } else {
        Bound::Exact
    };
    ctx.tt
        .store(hash, ply, best_move, best_score, NO_STATIC_EVAL, 0, bound);

    Ok(best_score)
}

fn next_qmove(pos: &Position, moves: &mut MoveList, next: &mut usize, skip: Move) -> Option<Move> {
    while *next < moves.len() {
        let mut best = *next;
        let mut best_score = score_qmove(pos, moves.as_slice()[best]);

        for index in (*next + 1)..moves.len() {
            let score = score_qmove(pos, moves.as_slice()[index]);
            if score > best_score {
                best = index;
                best_score = score;
            }
        }

        moves.swap(*next, best);
        let mv = moves.as_slice()[*next];
        *next += 1;

        if mv == skip {
            continue;
        }

        return Some(mv);
    }

    None
}

#[inline(always)]
fn delta_prune_move(pos: &Position, mv: Move, static_eval: i32, alpha: i32) -> bool {
    if is_mate_score(alpha) || mv.is_promotion() {
        return false;
    }

    let captured = captured_piece(pos, mv);
    static_eval + PIECE_VALUES[captured.index()] + DELTA_MARGIN <= alpha
}

#[inline(always)]
fn see_prune_move(pos: &Position, mv: Move) -> bool {
    mv.is_capture()
        && !mv.is_promotion()
        && mv.kind() != MoveKind::EnPassant
        && !see_ge(pos, mv, 0)
}

#[inline(always)]
fn score_qmove(pos: &Position, mv: Move) -> i16 {
    let kind = (mv.0 >> 12) as u8;
    let mut score = 0;

    if (kind & 0x8) != 0 {
        score += PROMOTION_BASE + PIECE_VALUES[((kind & 0x3) as usize) + 1];
    }

    if (kind & 0x4) != 0 || kind == MoveKind::EnPassant as u8 {
        let attacker = pos
            .piece_at(mv.from())
            .map_or(Piece::Pawn, |(piece, _)| piece);
        let captured = captured_piece(pos, mv);

        score +=
            CAPTURE_BASE + PIECE_VALUES[captured.index()] * 16 - PIECE_VALUES[attacker.index()];
    }

    debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
    score as i16
}

#[inline(always)]
fn captured_piece(pos: &Position, mv: Move) -> Piece {
    if ((mv.0 >> 12) as u8) == MoveKind::EnPassant as u8 {
        Piece::Pawn
    } else {
        pos.piece_at(mv.to()).map_or(Piece::Pawn, |(piece, _)| piece)
    }
}

#[inline(always)]
const fn is_tactical_move(mv: Move) -> bool {
    let kind = (mv.0 >> 12) as u8;
    (kind & 0x4) != 0 || (kind & 0x8) != 0 || kind == MoveKind::EnPassant as u8
}

#[inline(always)]
const fn is_valid_encoded_move(mv: Move) -> bool {
    matches!((mv.0 >> 12) as u8, 0..=4 | 8..=15)
}

#[inline(always)]
#[must_use]
fn pack_static_eval(score: i32) -> i16 {
    debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
    score as i16
}

const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0];
const CAPTURE_BASE: i32 = 10_000;
const PROMOTION_BASE: i32 = 20_000;
const DELTA_MARGIN: i32 = 200;

#[cfg(test)]
mod tests {
    use super::*;
    use oopsmate_core::Square;

    fn square(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    #[test]
    fn delta_pruning_never_skips_promotions() {
        let pos = Position::from_fen("4k3/3P4/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        let promotion = Move::new(square("d7"), square("d8"), MoveKind::PromotionQueen);

        assert!(!delta_prune_move(&pos, promotion, -1000, 500));
    }

    #[test]
    fn delta_pruning_skips_hopeless_small_capture() {
        let pos = Position::from_fen("4k3/8/8/8/8/8/p7/R3K3 w - - 0 1").unwrap();
        let capture = Move::new(square("a1"), square("a2"), MoveKind::Capture);

        assert!(delta_prune_move(&pos, capture, -600, 0));
    }
}
