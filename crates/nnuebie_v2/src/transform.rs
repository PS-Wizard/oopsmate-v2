use crate::arch::{FT_PERMUTE_BLOCK_I16S, FT_PERMUTE_GROUP_I16S, FT_PERMUTE_INVERSE_ORDER};
use oopsmate_core::Color;

#[inline(always)]
fn transform_perspective(accumulation: &[i16], output: &mut [u8]) {
    let half = accumulation.len() / 2;
    debug_assert_eq!(output.len(), half);

    if half % FT_PERMUTE_GROUP_I16S != 0 {
        for index in 0..half {
            let left = i32::from(accumulation[index]).clamp(0, 254);
            let right = i32::from(accumulation[index + half]).clamp(0, 254);
            output[index] = ((left * right) >> 9) as u8;
        }
        return;
    }

    for group_start in (0..half).step_by(FT_PERMUTE_GROUP_I16S) {
        for logical_block in 0..FT_PERMUTE_INVERSE_ORDER.len() {
            let packed_block = FT_PERMUTE_INVERSE_ORDER[logical_block];
            let packed_start = group_start + packed_block * FT_PERMUTE_BLOCK_I16S;
            let output_start = group_start + logical_block * FT_PERMUTE_BLOCK_I16S;

            for lane in 0..FT_PERMUTE_BLOCK_I16S {
                let index = packed_start + lane;
                let left = i32::from(accumulation[index]).clamp(0, 254);
                let right = i32::from(accumulation[index + half]).clamp(0, 254);
                output[output_start + lane] = ((left * right) >> 9) as u8;
            }
        }
    }
}

pub fn transform_features(accumulation: [&[i16]; 2], side_to_move: Color, output: &mut [u8]) {
    let half = output.len() / 2;
    let stm = side_to_move.index();
    let opp = stm ^ 1;

    transform_perspective(accumulation[stm], &mut output[..half]);
    transform_perspective(accumulation[opp], &mut output[half..]);
}

#[cfg(test)]
mod tests {
    use super::transform_features;
    use oopsmate_core::Color;

    #[test]
    fn transform_clamps_and_multiplies_pairs() {
        let white = [128i16, 64, 64, 128];
        let black = [254i16, 100, 160, 2];
        let mut output = [0u8; 4];

        transform_features([&white, &black], Color::White, &mut output);

        assert_eq!(output[0], 16);
        assert_eq!(output[1], 16);
        assert_eq!(output[2], 79);
        assert_eq!(output[3], 0);
    }
}
