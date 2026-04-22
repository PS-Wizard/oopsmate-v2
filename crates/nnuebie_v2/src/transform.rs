use oopsmate_core::Color;

#[inline(always)]
fn transform_perspective(accumulation: &[i16], output: &mut [u8]) {
    let half = accumulation.len() / 2;
    debug_assert_eq!(output.len(), half);

    for index in 0..half {
        let left = i32::from(accumulation[index]).clamp(0, 127);
        let right = i32::from(accumulation[index + half]).clamp(0, 127);
        output[index] = ((left * right) >> 7) as u8;
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
        let white = [64i16, 32, 32, 64];
        let black = [127i16, 50, 80, 1];
        let mut output = [0u8; 4];

        transform_features([&white, &black], Color::White, &mut output);

        assert_eq!(output[0], 16);
        assert_eq!(output[1], 16);
        assert_eq!(output[2], 79);
        assert_eq!(output[3], 0);
    }
}
