use crate::arch::DENSE_CHUNK_SIZE;
use crate::network::DenseLayer;

#[inline(always)]
pub fn affine_forward(layer: &DenseLayer, input: &[u8], output: &mut [i32]) {
    debug_assert_eq!(layer.output_dims, output.len());
    debug_assert!(input.len() >= layer.input_dims);
    debug_assert_eq!(layer.padded_input_dims % DENSE_CHUNK_SIZE, 0);

    output.copy_from_slice(&layer.biases);

    let chunk_count = layer.padded_input_dims / DENSE_CHUNK_SIZE;
    for chunk in 0..chunk_count {
        let input_base = chunk * DENSE_CHUNK_SIZE;
        let weight_base = chunk * layer.output_dims * DENSE_CHUNK_SIZE;
        let v0 = i32::from(input[input_base]);
        let v1 = i32::from(input[input_base + 1]);
        let v2 = i32::from(input[input_base + 2]);
        let v3 = i32::from(input[input_base + 3]);

        for out_index in 0..layer.output_dims {
            let offset = weight_base + out_index * DENSE_CHUNK_SIZE;
            output[out_index] += i32::from(layer.weights[offset]) * v0
                + i32::from(layer.weights[offset + 1]) * v1
                + i32::from(layer.weights[offset + 2]) * v2
                + i32::from(layer.weights[offset + 3]) * v3;
        }
    }
}
