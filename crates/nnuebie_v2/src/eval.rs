use crate::constants::{
    BIG_HALF_DIMS, BIG_NET_RECHECK_THRESHOLD, BISHOP_VALUE, FC0_OUTPUTS, FC1_OUTPUTS, KNIGHT_VALUE,
    OUTPUT_SCALE, PAWN_VALUE, PSQT_BUCKETS, QUEEN_VALUE, ROOK_VALUE,
    SMALLNET_SIMPLE_EVAL_THRESHOLD, VALUE_TB_LOSS_IN_MAX_PLY, VALUE_TB_WIN_IN_MAX_PLY,
};
use crate::context::NnueContext;
use crate::features::enumerate_active_features;
use crate::layers::{affine_forward, clipped_relu, sparse_affine_forward, sqr_clipped_relu};
use crate::network::{LoadedNetwork, NnueNetworks};
use crate::transform::transform_features;
use oopsmate_core::{Color, Piece, Position};

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
    #[must_use]
    pub fn evaluate(&self, position: &Position, ctx: &mut NnueContext) -> EvalOutput {
        let inputs = self.position_inputs(position);
        if inputs.piece_count == 0 {
            return EvalOutput::ZERO;
        }

        enumerate_active_features(position, &mut ctx.active_indices, &mut ctx.active_lengths);

        let bucket = ((usize::from(inputs.piece_count) - 1) / 4).min(PSQT_BUCKETS - 1);
        let mut used_smallnet =
            simple_eval(position, inputs.side_to_move).abs() > SMALLNET_SIMPLE_EVAL_THRESHOLD;

        let (mut psqt, mut positional) = if used_smallnet {
            evaluate_loaded_network(&self.small, ctx, inputs.side_to_move, bucket)
        } else {
            evaluate_loaded_network(&self.big, ctx, inputs.side_to_move, bucket)
        };

        let mut nnue = blended_nnue(psqt, positional);

        if used_smallnet && nnue.abs() < BIG_NET_RECHECK_THRESHOLD {
            (psqt, positional) =
                evaluate_loaded_network(&self.big, ctx, inputs.side_to_move, bucket);
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

fn evaluate_loaded_network(
    network: &LoadedNetwork,
    ctx: &mut NnueContext,
    side_to_move: Color,
    bucket: usize,
) -> (i32, i32) {
    if network.half_dims == BIG_HALF_DIMS {
        refresh_accumulator(
            network,
            &ctx.active_indices,
            &ctx.active_lengths,
            &mut ctx.big_accumulation,
            &mut ctx.big_psqt,
        );
        transform_features(
            [&ctx.big_accumulation[0], &ctx.big_accumulation[1]],
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
        let psqt_raw = (ctx.big_psqt[stm][bucket] - ctx.big_psqt[opp][bucket]) / 2;
        (psqt_raw / OUTPUT_SCALE, positional_raw / OUTPUT_SCALE)
    } else {
        refresh_accumulator(
            network,
            &ctx.active_indices,
            &ctx.active_lengths,
            &mut ctx.small_accumulation,
            &mut ctx.small_psqt,
        );
        transform_features(
            [&ctx.small_accumulation[0], &ctx.small_accumulation[1]],
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
        let psqt_raw = (ctx.small_psqt[stm][bucket] - ctx.small_psqt[opp][bucket]) / 2;
        (psqt_raw / OUTPUT_SCALE, positional_raw / OUTPUT_SCALE)
    }
}

fn refresh_accumulator<const HALF_DIMS: usize>(
    network: &LoadedNetwork,
    active_indices: &[[u32; crate::constants::MAX_ACTIVE_FEATURES]; 2],
    active_lengths: &[usize; 2],
    accumulation: &mut [[i16; HALF_DIMS]; 2],
    psqt: &mut [[i32; PSQT_BUCKETS]; 2],
) {
    let ft = &network.feature_transformer;

    for perspective in 0..2 {
        accumulation[perspective].copy_from_slice(&ft.biases);
        psqt[perspective].fill(0);

        for &feature_index in &active_indices[perspective][..active_lengths[perspective]] {
            let feature_index = feature_index as usize;
            let weight_row = feature_index * HALF_DIMS;
            let psqt_row = feature_index * PSQT_BUCKETS;

            for dim in 0..HALF_DIMS {
                accumulation[perspective][dim] =
                    accumulation[perspective][dim].wrapping_add(ft.weights[weight_row + dim]);
            }

            for bucket in 0..PSQT_BUCKETS {
                psqt[perspective][bucket] += ft.psqt_weights[psqt_row + bucket];
            }
        }
    }
}

fn propagate_loaded_network(
    stack: &crate::network::LayerStack,
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
    let residual_forward =
        residual * (600 * OUTPUT_SCALE) / (127 * (1 << crate::constants::WEIGHT_SCALE_BITS));

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
    use oopsmate_core::{Color, Position};
    use std::sync::OnceLock;

    static NETWORKS: OnceLock<NnueNetworks> = OnceLock::new();

    fn networks() -> &'static NnueNetworks {
        NETWORKS.get_or_init(|| NnueNetworks::load_default().expect("load default networks"))
    }

    fn white_side_cp(output: EvalOutput, position: &Position) -> i32 {
        output.white_side_cp(position.side_to_move())
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
}
