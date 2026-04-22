use crate::constants::{
    BIG_HALF_DIMS, FC0_TOTAL_OUTPUTS, FC1_OUTPUTS, FC1_PADDED_INPUT_DIMS, MAX_ACTIVE_FEATURES,
    PSQT_BUCKETS, SMALL_HALF_DIMS,
};

#[derive(Debug)]
pub struct NnueContext {
    pub(crate) active_indices: [[u32; MAX_ACTIVE_FEATURES]; 2],
    pub(crate) active_lengths: [usize; 2],
    pub(crate) big_accumulation: [[i16; BIG_HALF_DIMS]; 2],
    pub(crate) big_psqt: [[i32; PSQT_BUCKETS]; 2],
    pub(crate) small_accumulation: [[i16; SMALL_HALF_DIMS]; 2],
    pub(crate) small_psqt: [[i32; PSQT_BUCKETS]; 2],
    pub(crate) big_transformed: [u8; BIG_HALF_DIMS],
    pub(crate) small_transformed: [u8; SMALL_HALF_DIMS],
    pub(crate) fc0_out: [i32; FC0_TOTAL_OUTPUTS],
    pub(crate) fc1_in: [u8; FC1_PADDED_INPUT_DIMS],
    pub(crate) fc1_out: [i32; FC1_OUTPUTS],
    pub(crate) fc1_activated: [u8; FC1_OUTPUTS],
}

impl Default for NnueContext {
    fn default() -> Self {
        Self::new()
    }
}

impl NnueContext {
    #[inline(always)]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active_indices: [[0; MAX_ACTIVE_FEATURES]; 2],
            active_lengths: [0; 2],
            big_accumulation: [[0; BIG_HALF_DIMS]; 2],
            big_psqt: [[0; PSQT_BUCKETS]; 2],
            small_accumulation: [[0; SMALL_HALF_DIMS]; 2],
            small_psqt: [[0; PSQT_BUCKETS]; 2],
            big_transformed: [0; BIG_HALF_DIMS],
            small_transformed: [0; SMALL_HALF_DIMS],
            fc0_out: [0; FC0_TOTAL_OUTPUTS],
            fc1_in: [0; FC1_PADDED_INPUT_DIMS],
            fc1_out: [0; FC1_OUTPUTS],
            fc1_activated: [0; FC1_OUTPUTS],
        }
    }
}
