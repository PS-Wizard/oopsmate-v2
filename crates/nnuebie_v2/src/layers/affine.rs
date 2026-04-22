use crate::network::DenseLayer;

#[inline(always)]
pub fn affine_forward(layer: &DenseLayer, input: &[u8], output: &mut [i32]) {
    debug_assert_eq!(layer.output_dims, output.len());
    debug_assert!(input.len() >= layer.input_dims);

    output.copy_from_slice(&layer.biases);

    for out_index in 0..layer.output_dims {
        let row_offset = out_index * layer.padded_input_dims;
        let mut sum = output[out_index];

        for in_index in 0..layer.input_dims {
            sum += i32::from(layer.weights[row_offset + in_index]) * i32::from(input[in_index]);
        }

        output[out_index] = sum;
    }
}
