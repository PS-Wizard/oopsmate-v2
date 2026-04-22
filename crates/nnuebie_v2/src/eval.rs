use crate::constants::{
    BIG_HALF_DIMS, BIG_NET_RECHECK_THRESHOLD, BISHOP_VALUE, FC0_OUTPUTS, FC1_OUTPUTS, KNIGHT_VALUE,
    OUTPUT_SCALE, PAWN_VALUE, PSQT_BUCKETS, QUEEN_VALUE, ROOK_VALUE, SMALL_HALF_DIMS,
    SMALLNET_SIMPLE_EVAL_THRESHOLD, VALUE_TB_LOSS_IN_MAX_PLY, VALUE_TB_WIN_IN_MAX_PLY,
    WEIGHT_SCALE_BITS,
};
use crate::context::{AccumulatorFrame, DirtyPiece, NnueContext};
use crate::features::feature_index_from_piece_code;
use crate::layers::{affine_forward, clipped_relu, sparse_affine_forward, sqr_clipped_relu};
use crate::network::{FeatureTransformer, LayerStack, LoadedNetwork, NnueNetworks};
use crate::transform::transform_features;
use oopsmate_core::{Color, Piece, Position, Square};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EvalOutput {
    pub psqt: i32,
    pub positional: i32,
    pub final_raw: i32,
    pub final_cp: i32,
    pub used_smallnet: bool,
}

impl EvalOutput {
    pub const ZERO: Self = Self {
        psqt: 0,
        positional: 0,
        final_raw: 0,
        final_cp: 0,
        used_smallnet: false,
    };

    #[inline(always)]
    #[must_use]
    pub const fn white_side_cp(self, side_to_move: Color) -> i32 {
        if matches!(side_to_move, Color::Black) {
            -self.final_cp
        } else {
            self.final_cp
        }
    }
}

impl NnueNetworks {
    pub fn reset_context(&self, position: &Position, ctx: &mut NnueContext) {
        ctx.reset_root_state(position);
        full_refresh_frame_big(&self.big, position, &mut ctx.frames[0]);
        full_refresh_frame_small(&self.small, position, &mut ctx.frames[0]);
    }

    #[must_use]
    pub fn evaluate(&self, position: &Position, ctx: &mut NnueContext) -> EvalOutput {
        let inputs = self.position_inputs(position);
        if inputs.piece_count == 0 {
            return EvalOutput::ZERO;
        }

        if !ctx.initialized {
            debug_assert!(
                ctx.depth == 0,
                "NNUE context is uninitialized while inside the move stack"
            );
            self.reset_context(position, ctx);
        } else if ctx.depth == 0 && ctx.root_hash != position.hash() {
            self.reset_context(position, ctx);
        }

        let bucket = ((usize::from(inputs.piece_count) - 1) / 4).min(PSQT_BUCKETS - 1);
        let mut used_smallnet =
            simple_eval(position, inputs.side_to_move).abs() > SMALLNET_SIMPLE_EVAL_THRESHOLD;

        let (mut psqt, mut positional) = if used_smallnet {
            evaluate_small_network(&self.small, position, ctx, inputs.side_to_move, bucket)
        } else {
            evaluate_big_network(&self.big, position, ctx, inputs.side_to_move, bucket)
        };

        let mut nnue = blended_nnue(psqt, positional);

        if used_smallnet && nnue.abs() < BIG_NET_RECHECK_THRESHOLD {
            (psqt, positional) =
                evaluate_big_network(&self.big, position, ctx, inputs.side_to_move, bucket);
            nnue = blended_nnue(psqt, positional);
            used_smallnet = false;
        }

        let complexity = (psqt - positional).abs();
        nnue -= nnue * complexity / 18_000;

        let material = total_material_for_scaling(position);
        let mut final_raw = nnue * (77_777 + material) / 77_777;
        final_raw -= final_raw * i32::from(inputs.rule50) / 212;
        final_raw = final_raw.clamp(VALUE_TB_LOSS_IN_MAX_PLY + 1, VALUE_TB_WIN_IN_MAX_PLY - 1);

        EvalOutput {
            psqt,
            positional,
            final_raw,
            final_cp: to_centipawns(final_raw, position),
            used_smallnet,
        }
    }
}

#[inline(always)]
fn blended_nnue(psqt: i32, positional: i32) -> i32 {
    (125 * psqt + 131 * positional) / 128
}

fn evaluate_big_network(
    network: &LoadedNetwork,
    position: &Position,
    ctx: &mut NnueContext,
    side_to_move: Color,
    bucket: usize,
) -> (i32, i32) {
    ensure_big_frame(network, position, ctx);

    let depth = ctx.depth;
    let frame = &ctx.frames[depth];
    transform_features(
        [&frame.big_accumulation[0], &frame.big_accumulation[1]],
        side_to_move,
        &mut ctx.big_transformed,
    );
    let positional_raw = propagate_loaded_network(
        &network.layer_stacks[bucket],
        &ctx.big_transformed,
        &mut ctx.fc0_out,
        &mut ctx.fc1_in,
        &mut ctx.fc1_out,
        &mut ctx.fc1_activated,
    );
    let stm = side_to_move.index();
    let opp = stm ^ 1;
    let psqt_raw = (frame.big_psqt[stm][bucket] - frame.big_psqt[opp][bucket]) / 2;
    (psqt_raw / OUTPUT_SCALE, positional_raw / OUTPUT_SCALE)
}

fn evaluate_small_network(
    network: &LoadedNetwork,
    position: &Position,
    ctx: &mut NnueContext,
    side_to_move: Color,
    bucket: usize,
) -> (i32, i32) {
    ensure_small_frame(network, position, ctx);

    let depth = ctx.depth;
    let frame = &ctx.frames[depth];
    transform_features(
        [&frame.small_accumulation[0], &frame.small_accumulation[1]],
        side_to_move,
        &mut ctx.small_transformed,
    );
    let positional_raw = propagate_loaded_network(
        &network.layer_stacks[bucket],
        &ctx.small_transformed,
        &mut ctx.fc0_out,
        &mut ctx.fc1_in,
        &mut ctx.fc1_out,
        &mut ctx.fc1_activated,
    );
    let stm = side_to_move.index();
    let opp = stm ^ 1;
    let psqt_raw = (frame.small_psqt[stm][bucket] - frame.small_psqt[opp][bucket]) / 2;
    (psqt_raw / OUTPUT_SCALE, positional_raw / OUTPUT_SCALE)
}

fn ensure_big_frame(network: &LoadedNetwork, position: &Position, ctx: &mut NnueContext) {
    ensure_big_perspective(network, position, ctx, Color::White);
    ensure_big_perspective(network, position, ctx, Color::Black);
}

fn ensure_small_frame(network: &LoadedNetwork, position: &Position, ctx: &mut NnueContext) {
    ensure_small_perspective(network, position, ctx, Color::White);
    ensure_small_perspective(network, position, ctx, Color::Black);
}

fn ensure_big_perspective(
    network: &LoadedNetwork,
    position: &Position,
    ctx: &mut NnueContext,
    perspective: Color,
) {
    let p = perspective.index();
    let current = ctx.depth;
    if ctx.frames[current].big_computed[p] {
        return;
    }

    match find_last_usable_big(ctx, perspective) {
        LastUsable::Computed(begin) => {
            let king_square = position.board().king_square(perspective);
            for next in (begin + 1)..=current {
                let (left, right) = ctx.frames.split_at_mut(next);
                let source = &left[next - 1];
                let target = &mut right[0];
                forward_update_big(
                    &network.feature_transformer,
                    perspective,
                    king_square,
                    source,
                    target,
                );
            }
        }
        LastUsable::RefreshBoundary(boundary) => {
            full_refresh_big_side(network, position, &mut ctx.frames[current], perspective);

            if boundary < current {
                let king_square = position.board().king_square(perspective);
                for prev in (boundary..current).rev() {
                    let (left, right) = ctx.frames.split_at_mut(prev + 1);
                    let target = &mut left[prev];
                    let source = &right[0];
                    backward_update_big(
                        &network.feature_transformer,
                        perspective,
                        king_square,
                        source,
                        target,
                    );
                }
            }
        }
    }
}

fn ensure_small_perspective(
    network: &LoadedNetwork,
    position: &Position,
    ctx: &mut NnueContext,
    perspective: Color,
) {
    let p = perspective.index();
    let current = ctx.depth;
    if ctx.frames[current].small_computed[p] {
        return;
    }

    match find_last_usable_small(ctx, perspective) {
        LastUsable::Computed(begin) => {
            let king_square = position.board().king_square(perspective);
            for next in (begin + 1)..=current {
                let (left, right) = ctx.frames.split_at_mut(next);
                let source = &left[next - 1];
                let target = &mut right[0];
                forward_update_small(
                    &network.feature_transformer,
                    perspective,
                    king_square,
                    source,
                    target,
                );
            }
        }
        LastUsable::RefreshBoundary(boundary) => {
            full_refresh_small_side(network, position, &mut ctx.frames[current], perspective);

            if boundary < current {
                let king_square = position.board().king_square(perspective);
                for prev in (boundary..current).rev() {
                    let (left, right) = ctx.frames.split_at_mut(prev + 1);
                    let target = &mut left[prev];
                    let source = &right[0];
                    backward_update_small(
                        &network.feature_transformer,
                        perspective,
                        king_square,
                        source,
                        target,
                    );
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastUsable {
    Computed(usize),
    RefreshBoundary(usize),
}

fn find_last_usable_big(ctx: &NnueContext, perspective: Color) -> LastUsable {
    let p = perspective.index();
    for idx in (1..=ctx.depth).rev() {
        if ctx.frames[idx].big_computed[p] {
            return LastUsable::Computed(idx);
        }
        if ctx.frames[idx].dirty.requires_refresh(perspective) {
            return LastUsable::RefreshBoundary(idx);
        }
    }

    if ctx.frames[0].big_computed[p] {
        LastUsable::Computed(0)
    } else {
        LastUsable::RefreshBoundary(0)
    }
}

fn find_last_usable_small(ctx: &NnueContext, perspective: Color) -> LastUsable {
    let p = perspective.index();
    for idx in (1..=ctx.depth).rev() {
        if ctx.frames[idx].small_computed[p] {
            return LastUsable::Computed(idx);
        }
        if ctx.frames[idx].dirty.requires_refresh(perspective) {
            return LastUsable::RefreshBoundary(idx);
        }
    }

    if ctx.frames[0].small_computed[p] {
        LastUsable::Computed(0)
    } else {
        LastUsable::RefreshBoundary(0)
    }
}

fn full_refresh_frame_big(
    network: &LoadedNetwork,
    position: &Position,
    frame: &mut AccumulatorFrame,
) {
    full_refresh_big_side(network, position, frame, Color::White);
    full_refresh_big_side(network, position, frame, Color::Black);
}

fn full_refresh_frame_small(
    network: &LoadedNetwork,
    position: &Position,
    frame: &mut AccumulatorFrame,
) {
    full_refresh_small_side(network, position, frame, Color::White);
    full_refresh_small_side(network, position, frame, Color::Black);
}

fn full_refresh_big_side(
    network: &LoadedNetwork,
    position: &Position,
    frame: &mut AccumulatorFrame,
    perspective: Color,
) {
    let p = perspective.index();
    full_refresh_side::<BIG_HALF_DIMS>(
        &network.feature_transformer,
        position,
        perspective,
        &mut frame.big_accumulation[p],
        &mut frame.big_psqt[p],
    );
    frame.big_computed[p] = true;
}

fn full_refresh_small_side(
    network: &LoadedNetwork,
    position: &Position,
    frame: &mut AccumulatorFrame,
    perspective: Color,
) {
    let p = perspective.index();
    full_refresh_side::<SMALL_HALF_DIMS>(
        &network.feature_transformer,
        position,
        perspective,
        &mut frame.small_accumulation[p],
        &mut frame.small_psqt[p],
    );
    frame.small_computed[p] = true;
}

fn full_refresh_side<const HALF_DIMS: usize>(
    feature_transformer: &FeatureTransformer,
    position: &Position,
    perspective: Color,
    accumulation: &mut [i16; HALF_DIMS],
    psqt: &mut [i32; PSQT_BUCKETS],
) {
    accumulation.copy_from_slice(&feature_transformer.biases);
    psqt.fill(0);

    let board = position.board();
    let king_square = board.king_square(perspective);

    for (square, &piece_code) in board.squares().iter().enumerate() {
        if piece_code == oopsmate_core::EMPTY_SQUARE {
            continue;
        }

        let feature_index = feature_index_from_piece_code(
            perspective,
            piece_code,
            Square::from_raw(square as u8),
            king_square,
        ) as usize;
        apply_feature_add::<HALF_DIMS>(feature_transformer, feature_index, accumulation, psqt);
    }
}

fn forward_update_big(
    feature_transformer: &FeatureTransformer,
    perspective: Color,
    king_square: Square,
    source: &AccumulatorFrame,
    target: &mut AccumulatorFrame,
) {
    apply_dirty_update::<BIG_HALF_DIMS>(
        feature_transformer,
        target.dirty,
        perspective,
        king_square,
        &source.big_accumulation[perspective.index()],
        &source.big_psqt[perspective.index()],
        &mut target.big_accumulation[perspective.index()],
        &mut target.big_psqt[perspective.index()],
        true,
    );
    target.big_computed[perspective.index()] = true;
}

fn backward_update_big(
    feature_transformer: &FeatureTransformer,
    perspective: Color,
    king_square: Square,
    source: &AccumulatorFrame,
    target: &mut AccumulatorFrame,
) {
    apply_dirty_update::<BIG_HALF_DIMS>(
        feature_transformer,
        source.dirty,
        perspective,
        king_square,
        &source.big_accumulation[perspective.index()],
        &source.big_psqt[perspective.index()],
        &mut target.big_accumulation[perspective.index()],
        &mut target.big_psqt[perspective.index()],
        false,
    );
    target.big_computed[perspective.index()] = true;
}

fn forward_update_small(
    feature_transformer: &FeatureTransformer,
    perspective: Color,
    king_square: Square,
    source: &AccumulatorFrame,
    target: &mut AccumulatorFrame,
) {
    apply_dirty_update::<SMALL_HALF_DIMS>(
        feature_transformer,
        target.dirty,
        perspective,
        king_square,
        &source.small_accumulation[perspective.index()],
        &source.small_psqt[perspective.index()],
        &mut target.small_accumulation[perspective.index()],
        &mut target.small_psqt[perspective.index()],
        true,
    );
    target.small_computed[perspective.index()] = true;
}

fn backward_update_small(
    feature_transformer: &FeatureTransformer,
    perspective: Color,
    king_square: Square,
    source: &AccumulatorFrame,
    target: &mut AccumulatorFrame,
) {
    apply_dirty_update::<SMALL_HALF_DIMS>(
        feature_transformer,
        source.dirty,
        perspective,
        king_square,
        &source.small_accumulation[perspective.index()],
        &source.small_psqt[perspective.index()],
        &mut target.small_accumulation[perspective.index()],
        &mut target.small_psqt[perspective.index()],
        false,
    );
    target.small_computed[perspective.index()] = true;
}

fn apply_dirty_update<const HALF_DIMS: usize>(
    feature_transformer: &FeatureTransformer,
    dirty: DirtyPiece,
    perspective: Color,
    king_square: Square,
    source_accumulation: &[i16; HALF_DIMS],
    source_psqt: &[i32; PSQT_BUCKETS],
    target_accumulation: &mut [i16; HALF_DIMS],
    target_psqt: &mut [i32; PSQT_BUCKETS],
    forward: bool,
) {
    target_accumulation.copy_from_slice(source_accumulation);
    target_psqt.copy_from_slice(source_psqt);

    for idx in 0..dirty.len {
        let piece_code = dirty.piece_codes[idx];
        let removed = if forward {
            dirty.from[idx]
        } else {
            dirty.to[idx]
        };
        let added = if forward {
            dirty.to[idx]
        } else {
            dirty.from[idx]
        };

        if removed.is_valid() {
            let feature_index =
                feature_index_from_piece_code(perspective, piece_code, removed, king_square)
                    as usize;
            apply_feature_sub::<HALF_DIMS>(
                feature_transformer,
                feature_index,
                target_accumulation,
                target_psqt,
            );
        }

        if added.is_valid() {
            let feature_index =
                feature_index_from_piece_code(perspective, piece_code, added, king_square) as usize;
            apply_feature_add::<HALF_DIMS>(
                feature_transformer,
                feature_index,
                target_accumulation,
                target_psqt,
            );
        }
    }
}

#[inline(always)]
fn apply_feature_add<const HALF_DIMS: usize>(
    feature_transformer: &FeatureTransformer,
    feature_index: usize,
    accumulation: &mut [i16; HALF_DIMS],
    psqt: &mut [i32; PSQT_BUCKETS],
) {
    let weight_row = feature_index * HALF_DIMS;
    let psqt_row = feature_index * PSQT_BUCKETS;

    for dim in 0..HALF_DIMS {
        accumulation[dim] =
            accumulation[dim].wrapping_add(feature_transformer.weights[weight_row + dim]);
    }

    for bucket in 0..PSQT_BUCKETS {
        psqt[bucket] += feature_transformer.psqt_weights[psqt_row + bucket];
    }
}

#[inline(always)]
fn apply_feature_sub<const HALF_DIMS: usize>(
    feature_transformer: &FeatureTransformer,
    feature_index: usize,
    accumulation: &mut [i16; HALF_DIMS],
    psqt: &mut [i32; PSQT_BUCKETS],
) {
    let weight_row = feature_index * HALF_DIMS;
    let psqt_row = feature_index * PSQT_BUCKETS;

    for dim in 0..HALF_DIMS {
        accumulation[dim] =
            accumulation[dim].wrapping_sub(feature_transformer.weights[weight_row + dim]);
    }

    for bucket in 0..PSQT_BUCKETS {
        psqt[bucket] -= feature_transformer.psqt_weights[psqt_row + bucket];
    }
}

fn propagate_loaded_network(
    stack: &LayerStack,
    transformed: &[u8],
    fc0_out: &mut [i32; crate::constants::FC0_TOTAL_OUTPUTS],
    fc1_in: &mut [u8; crate::constants::FC1_PADDED_INPUT_DIMS],
    fc1_out: &mut [i32; crate::constants::FC1_OUTPUTS],
    fc1_activated: &mut [u8; crate::constants::FC1_OUTPUTS],
) -> i32 {
    sparse_affine_forward(&stack.fc0, transformed, fc0_out);

    fc1_in.fill(0);
    sqr_clipped_relu(&fc0_out[..FC0_OUTPUTS], &mut fc1_in[..FC0_OUTPUTS]);
    clipped_relu(
        &fc0_out[..FC0_OUTPUTS],
        &mut fc1_in[FC0_OUTPUTS..FC0_OUTPUTS * 2],
    );

    affine_forward(&stack.fc1, fc1_in, fc1_out);
    clipped_relu(fc1_out, fc1_activated);

    let mut positional = stack.fc2.biases[0];
    for index in 0..FC1_OUTPUTS {
        positional += i32::from(stack.fc2.weights[index]) * i32::from(fc1_activated[index]);
    }

    let residual = fc0_out[FC0_OUTPUTS];
    let residual_forward = residual * (600 * OUTPUT_SCALE) / (127 * (1 << WEIGHT_SCALE_BITS));

    positional + residual_forward
}

fn simple_eval(position: &Position, side_to_move: Color) -> i32 {
    PAWN_VALUE * (pawn_count(position, side_to_move) - pawn_count(position, side_to_move.flip()))
        + (non_pawn_material(position, side_to_move)
            - non_pawn_material(position, side_to_move.flip()))
}

fn pawn_count(position: &Position, color: Color) -> i32 {
    let board = position.board();
    (board.piece_bb(Piece::Pawn) & board.color_bb(color)).count_ones() as i32
}

fn non_pawn_material(position: &Position, color: Color) -> i32 {
    let board = position.board();
    let color_bb = board.color_bb(color);

    ((board.piece_bb(Piece::Knight) & color_bb).count_ones() as i32 * KNIGHT_VALUE)
        + ((board.piece_bb(Piece::Bishop) & color_bb).count_ones() as i32 * BISHOP_VALUE)
        + ((board.piece_bb(Piece::Rook) & color_bb).count_ones() as i32 * ROOK_VALUE)
        + ((board.piece_bb(Piece::Queen) & color_bb).count_ones() as i32 * QUEEN_VALUE)
}

fn total_material_for_scaling(position: &Position) -> i32 {
    let board = position.board();
    535 * board.piece_bb(Piece::Pawn).count_ones() as i32
        + board.piece_bb(Piece::Knight).count_ones() as i32 * KNIGHT_VALUE
        + board.piece_bb(Piece::Bishop).count_ones() as i32 * BISHOP_VALUE
        + board.piece_bb(Piece::Rook).count_ones() as i32 * ROOK_VALUE
        + board.piece_bb(Piece::Queen).count_ones() as i32 * QUEEN_VALUE
}

fn coarse_material_for_cp(position: &Position) -> i32 {
    let board = position.board();
    board.piece_bb(Piece::Pawn).count_ones() as i32
        + 3 * board.piece_bb(Piece::Knight).count_ones() as i32
        + 3 * board.piece_bb(Piece::Bishop).count_ones() as i32
        + 5 * board.piece_bb(Piece::Rook).count_ones() as i32
        + 9 * board.piece_bb(Piece::Queen).count_ones() as i32
}

fn to_centipawns(value: i32, position: &Position) -> i32 {
    let material = coarse_material_for_cp(position);
    let m = material.clamp(17, 78) as f64 / 58.0;

    let a = (((-13.50030198 * m + 40.92780883) * m - 36.82753545) * m) + 386.83004070;

    (100.0 * f64::from(value) / a).round() as i32
}

#[cfg(test)]
mod tests {
    use super::EvalOutput;
    use crate::{NnueContext, NnueNetworks};
    use oopsmate_core::{Color, Move, MoveKind, Position, Square};
    use std::sync::OnceLock;

    static NETWORKS: OnceLock<NnueNetworks> = OnceLock::new();

    fn networks() -> &'static NnueNetworks {
        NETWORKS.get_or_init(|| NnueNetworks::load_default().expect("load default networks"))
    }

    fn white_side_cp(output: EvalOutput, position: &Position) -> i32 {
        output.white_side_cp(position.side_to_move())
    }

    fn sq(text: &str) -> Square {
        Square::from_algebraic(text).unwrap()
    }

    fn assert_incremental_matches_full(
        networks: &NnueNetworks,
        position: &Position,
        incremental_ctx: &mut NnueContext,
    ) {
        let incremental = networks.evaluate(position, incremental_ctx);

        let mut full_ctx = NnueContext::new();
        networks.reset_context(position, &mut full_ctx);
        let full = networks.evaluate(position, &mut full_ctx);

        assert_eq!(incremental.psqt, full.psqt);
        assert_eq!(incremental.positional, full.positional);
        assert_eq!(incremental.final_raw, full.final_raw);
        assert_eq!(incremental.final_cp, full.final_cp);
        assert_eq!(incremental.used_smallnet, full.used_smallnet);
    }

    fn walk_limited_tree(
        networks: &NnueNetworks,
        position: &mut Position,
        incremental_ctx: &mut NnueContext,
        depth: usize,
        branch_limit: usize,
    ) {
        assert_incremental_matches_full(networks, position, incremental_ctx);

        if depth == 0 {
            return;
        }

        let mut moves = oopsmate_movegen::MoveList::new();
        oopsmate_movegen::generate_all(position, &mut moves);

        for &mv in moves.as_slice().iter().take(branch_limit) {
            incremental_ctx.push_move(position, mv);
            position.make_move(mv);
            walk_limited_tree(networks, position, incremental_ctx, depth - 1, branch_limit);
            position.unmake_move(mv);
            incremental_ctx.pop();
        }
    }

    #[test]
    fn validate_reference_positions() {
        let cases = [
            (
                "Startpos",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                7,
            ),
            (
                "King Triggers Refresh",
                "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
                -20,
            ),
            (
                "e4",
                "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
                37,
            ),
            (
                "No Queen",
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1",
                -522,
            ),
            (
                "Opening",
                "r1bqkb1r/pppp1ppp/2n2n2/3Pp3/4P3/2N2N2/PPP2PPP/R1BQKB1R b KQkq - 0 1",
                113,
            ),
            (
                "Middlegame 1",
                "r1bq1rk1/ppp1npbp/2np2p1/4p3/2P4N/2NP2P1/PP2PPBP/R1BQ1RK1 w - - 0 1",
                4,
            ),
            (
                "Middlegame 2",
                "r1bq1rk1/1pp2pbN/2np4/4p3/7N/3P2P1/1P2PPBP/R1BQ1RK1 w - - 0 1",
                389,
            ),
        ];

        let networks = networks();
        let mut ctx = NnueContext::new();

        for (name, fen, expected_cp) in cases {
            let position = Position::from_fen(fen).expect(name);
            let output = networks.evaluate(&position, &mut ctx);
            assert_eq!(white_side_cp(output, &position), expected_cp, "{name}");
        }
    }

    #[test]
    fn white_side_cp_flips_black_positions() {
        let output = EvalOutput {
            final_cp: 37,
            ..EvalOutput::ZERO
        };

        assert_eq!(output.white_side_cp(Color::White), 37);
        assert_eq!(output.white_side_cp(Color::Black), -37);
    }

    #[test]
    fn incremental_matches_full_on_limited_startpos_tree() {
        let networks = networks();
        let mut position = Position::startpos();
        let mut incremental_ctx = NnueContext::new();
        networks.reset_context(&position, &mut incremental_ctx);

        walk_limited_tree(networks, &mut position, &mut incremental_ctx, 3, 6);
    }

    #[test]
    fn incremental_matches_full_on_castle_en_passant_and_promotion_sequences() {
        let networks = networks();

        let sequences = [
            (
                "castle",
                "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
                vec![Move::new(sq("e1"), sq("g1"), MoveKind::Castle)],
            ),
            (
                "en-passant",
                "8/8/8/3pP3/8/8/8/4K2k w - d6 0 1",
                vec![Move::new(sq("e5"), sq("d6"), MoveKind::EnPassant)],
            ),
            (
                "promotion",
                "4k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                vec![Move::new(sq("a7"), sq("a8"), MoveKind::PromotionQueen)],
            ),
            (
                "capture-promotion",
                "1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1",
                vec![Move::new(
                    sq("a7"),
                    sq("b8"),
                    MoveKind::CapturePromotionQueen,
                )],
            ),
        ];

        for (name, fen, sequence) in sequences {
            let mut position = Position::from_fen(fen).expect(name);
            let mut incremental_ctx = NnueContext::new();
            networks.reset_context(&position, &mut incremental_ctx);
            assert_incremental_matches_full(networks, &position, &mut incremental_ctx);

            for mv in sequence {
                incremental_ctx.push_move(&position, mv);
                position.make_move(mv);
                assert_incremental_matches_full(networks, &position, &mut incremental_ctx);
                position.unmake_move(mv);
                incremental_ctx.pop();
                assert_incremental_matches_full(networks, &position, &mut incremental_ctx);
            }
        }
    }
}
