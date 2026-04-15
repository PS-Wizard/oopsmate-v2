use crate::types::{EMPTY_SQUARE, Square};

const fn splitmix64_next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

const fn generate_piece_keys() -> [[u64; 64]; 12] {
    let mut seed = 0x1234_5678_9abc_def0;
    let mut keys = [[0u64; 64]; 12];
    let mut piece = 0;
    while piece < 12 {
        let mut square = 0;
        while square < 64 {
            keys[piece][square] = splitmix64_next(&mut seed);
            square += 1;
        }
        piece += 1;
    }
    keys
}

const fn generate_castling_keys() -> [u64; 16] {
    let mut seed = 0x0fed_cba9_8765_4321;
    let mut keys = [0u64; 16];
    let mut idx = 0;
    while idx < 16 {
        keys[idx] = splitmix64_next(&mut seed);
        idx += 1;
    }
    keys
}

const fn generate_ep_keys() -> [u64; 8] {
    let mut seed = 0x55aa_aa55_33cc_cc33;
    let mut keys = [0u64; 8];
    let mut idx = 0;
    while idx < 8 {
        keys[idx] = splitmix64_next(&mut seed);
        idx += 1;
    }
    keys
}

pub(crate) static PIECE_KEYS: [[u64; 64]; 12] = generate_piece_keys();
pub(crate) static CASTLING_KEYS: [u64; 16] = generate_castling_keys();
pub(crate) static EP_KEYS: [u64; 8] = generate_ep_keys();
pub(crate) static SIDE_KEY: u64 = 0xa1b2_c3d4_e5f6_0718;

#[inline(always)]
#[must_use]
pub(crate) const fn piece_key(piece_code: u8, square: Square) -> u64 {
    if piece_code == EMPTY_SQUARE {
        0
    } else {
        PIECE_KEYS[(piece_code - 1) as usize][square.index()]
    }
}

#[inline(always)]
#[must_use]
pub(crate) const fn castling_key(castling: u8) -> u64 {
    CASTLING_KEYS[castling as usize]
}

#[inline(always)]
#[must_use]
pub(crate) const fn ep_key(square: Square) -> u64 {
    EP_KEYS[square.file() as usize]
}
