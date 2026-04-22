use crate::network::DenseLayer;

#[inline(always)]
pub fn sparse_affine_forward(layer: &DenseLayer, input: &[u8], output: &mut [i32]) {
    debug_assert_eq!(layer.output_dims, output.len());
    debug_assert!(input.len() >= layer.input_dims);

    output.copy_from_slice(&layer.biases);

    for in_index in 0..layer.input_dims {
        let value = input[in_index];
        if value == 0 {
            continue;
        }

        let value = i32::from(value);
        for out_index in 0..layer.output_dims {
            let weight = i32::from(layer.weights[out_index * layer.padded_input_dims + in_index]);
            output[out_index] += weight * value;
        }
    }
}
