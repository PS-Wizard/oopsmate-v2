use crate::constants::PSQT_BUCKETS;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{
    __m256i, _mm256_add_epi16, _mm256_add_epi32, _mm256_load_si256, _mm256_store_si256,
    _mm256_sub_epi16, _mm256_sub_epi32,
};

#[inline(always)]
pub(crate) fn accum_add<const N: usize>(acc: &mut [i16; N], add: &[i16]) {
    debug_assert_eq!(add.len(), N);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i16(acc.as_ptr(), add.as_ptr(), N) {
        unsafe {
            accum_add_avx2(acc, add);
        }
        return;
    }

    for idx in 0..N {
        acc[idx] = acc[idx].wrapping_add(add[idx]);
    }
}

#[inline(always)]
pub(crate) fn accum_sub<const N: usize>(acc: &mut [i16; N], sub: &[i16]) {
    debug_assert_eq!(sub.len(), N);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i16(acc.as_ptr(), sub.as_ptr(), N) {
        unsafe {
            accum_sub_avx2(acc, sub);
        }
        return;
    }

    for idx in 0..N {
        acc[idx] = acc[idx].wrapping_sub(sub[idx]);
    }
}

#[inline(always)]
pub(crate) fn accum_add_sub<const N: usize>(acc: &mut [i16; N], add: &[i16], sub: &[i16]) {
    debug_assert_eq!(add.len(), N);
    debug_assert_eq!(sub.len(), N);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i16_3(acc.as_ptr(), add.as_ptr(), sub.as_ptr(), N) {
        unsafe {
            accum_add_sub_avx2(acc, add, sub);
        }
        return;
    }

    for idx in 0..N {
        acc[idx] = acc[idx].wrapping_add(add[idx]).wrapping_sub(sub[idx]);
    }
}

#[inline(always)]
pub(crate) fn accum_add1_sub1_into<const N: usize>(
    base: &[i16; N],
    add: &[i16],
    sub: &[i16],
    out: &mut [i16; N],
) {
    debug_assert_eq!(add.len(), N);
    debug_assert_eq!(sub.len(), N);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i16_4(base.as_ptr(), add.as_ptr(), sub.as_ptr(), out.as_ptr(), N) {
        unsafe {
            accum_add1_sub1_into_avx2(base, add, sub, out);
        }
        return;
    }

    for idx in 0..N {
        out[idx] = base[idx].wrapping_add(add[idx]).wrapping_sub(sub[idx]);
    }
}

#[inline(always)]
pub(crate) fn accum_add1_sub2_into<const N: usize>(
    base: &[i16; N],
    add: &[i16],
    sub0: &[i16],
    sub1: &[i16],
    out: &mut [i16; N],
) {
    debug_assert_eq!(add.len(), N);
    debug_assert_eq!(sub0.len(), N);
    debug_assert_eq!(sub1.len(), N);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i16_5(
        base.as_ptr(),
        add.as_ptr(),
        sub0.as_ptr(),
        sub1.as_ptr(),
        out.as_ptr(),
        N,
    ) {
        unsafe {
            accum_add1_sub2_into_avx2(base, add, sub0, sub1, out);
        }
        return;
    }

    for idx in 0..N {
        out[idx] = base[idx]
            .wrapping_add(add[idx])
            .wrapping_sub(sub0[idx])
            .wrapping_sub(sub1[idx]);
    }
}

#[inline(always)]
pub(crate) fn accum_add2_sub1_into<const N: usize>(
    base: &[i16; N],
    add0: &[i16],
    add1: &[i16],
    sub: &[i16],
    out: &mut [i16; N],
) {
    debug_assert_eq!(add0.len(), N);
    debug_assert_eq!(add1.len(), N);
    debug_assert_eq!(sub.len(), N);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i16_5(
        base.as_ptr(),
        add0.as_ptr(),
        add1.as_ptr(),
        sub.as_ptr(),
        out.as_ptr(),
        N,
    ) {
        unsafe {
            accum_add2_sub1_into_avx2(base, add0, add1, sub, out);
        }
        return;
    }

    for idx in 0..N {
        out[idx] = base[idx]
            .wrapping_add(add0[idx])
            .wrapping_add(add1[idx])
            .wrapping_sub(sub[idx]);
    }
}

#[inline(always)]
pub(crate) fn accum_add2_sub2_into<const N: usize>(
    base: &[i16; N],
    add0: &[i16],
    add1: &[i16],
    sub0: &[i16],
    sub1: &[i16],
    out: &mut [i16; N],
) {
    debug_assert_eq!(add0.len(), N);
    debug_assert_eq!(add1.len(), N);
    debug_assert_eq!(sub0.len(), N);
    debug_assert_eq!(sub1.len(), N);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i16_6(
        base.as_ptr(),
        add0.as_ptr(),
        add1.as_ptr(),
        sub0.as_ptr(),
        sub1.as_ptr(),
        out.as_ptr(),
        N,
    ) {
        unsafe {
            accum_add2_sub2_into_avx2(base, add0, add1, sub0, sub1, out);
        }
        return;
    }

    for idx in 0..N {
        out[idx] = base[idx]
            .wrapping_add(add0[idx])
            .wrapping_add(add1[idx])
            .wrapping_sub(sub0[idx])
            .wrapping_sub(sub1[idx]);
    }
}

#[inline(always)]
pub(crate) fn psqt_add(psqt: &mut [i32; PSQT_BUCKETS], add: &[i32]) {
    debug_assert_eq!(add.len(), PSQT_BUCKETS);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i32(psqt.as_ptr(), add.as_ptr()) {
        unsafe {
            psqt_add_avx2(psqt, add);
        }
        return;
    }

    for idx in 0..PSQT_BUCKETS {
        psqt[idx] += add[idx];
    }
}

#[inline(always)]
pub(crate) fn psqt_sub(psqt: &mut [i32; PSQT_BUCKETS], sub: &[i32]) {
    debug_assert_eq!(sub.len(), PSQT_BUCKETS);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i32(psqt.as_ptr(), sub.as_ptr()) {
        unsafe {
            psqt_sub_avx2(psqt, sub);
        }
        return;
    }

    for idx in 0..PSQT_BUCKETS {
        psqt[idx] -= sub[idx];
    }
}

#[inline(always)]
pub(crate) fn psqt_add_sub(psqt: &mut [i32; PSQT_BUCKETS], add: &[i32], sub: &[i32]) {
    debug_assert_eq!(add.len(), PSQT_BUCKETS);
    debug_assert_eq!(sub.len(), PSQT_BUCKETS);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i32_3(psqt.as_ptr(), add.as_ptr(), sub.as_ptr()) {
        unsafe {
            psqt_add_sub_avx2(psqt, add, sub);
        }
        return;
    }

    for idx in 0..PSQT_BUCKETS {
        psqt[idx] += add[idx] - sub[idx];
    }
}

#[inline(always)]
pub(crate) fn psqt_add1_sub1_into(
    base: &[i32; PSQT_BUCKETS],
    add: &[i32],
    sub: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    debug_assert_eq!(add.len(), PSQT_BUCKETS);
    debug_assert_eq!(sub.len(), PSQT_BUCKETS);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i32_4(base.as_ptr(), add.as_ptr(), sub.as_ptr(), out.as_ptr()) {
        unsafe {
            psqt_add1_sub1_into_avx2(base, add, sub, out);
        }
        return;
    }

    for idx in 0..PSQT_BUCKETS {
        out[idx] = base[idx] + add[idx] - sub[idx];
    }
}

#[inline(always)]
pub(crate) fn psqt_add1_sub2_into(
    base: &[i32; PSQT_BUCKETS],
    add: &[i32],
    sub0: &[i32],
    sub1: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    debug_assert_eq!(add.len(), PSQT_BUCKETS);
    debug_assert_eq!(sub0.len(), PSQT_BUCKETS);
    debug_assert_eq!(sub1.len(), PSQT_BUCKETS);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i32_5(
        base.as_ptr(),
        add.as_ptr(),
        sub0.as_ptr(),
        sub1.as_ptr(),
        out.as_ptr(),
    ) {
        unsafe {
            psqt_add1_sub2_into_avx2(base, add, sub0, sub1, out);
        }
        return;
    }

    for idx in 0..PSQT_BUCKETS {
        out[idx] = base[idx] + add[idx] - sub0[idx] - sub1[idx];
    }
}

#[inline(always)]
pub(crate) fn psqt_add2_sub1_into(
    base: &[i32; PSQT_BUCKETS],
    add0: &[i32],
    add1: &[i32],
    sub: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    debug_assert_eq!(add0.len(), PSQT_BUCKETS);
    debug_assert_eq!(add1.len(), PSQT_BUCKETS);
    debug_assert_eq!(sub.len(), PSQT_BUCKETS);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i32_5(
        base.as_ptr(),
        add0.as_ptr(),
        add1.as_ptr(),
        sub.as_ptr(),
        out.as_ptr(),
    ) {
        unsafe {
            psqt_add2_sub1_into_avx2(base, add0, add1, sub, out);
        }
        return;
    }

    for idx in 0..PSQT_BUCKETS {
        out[idx] = base[idx] + add0[idx] + add1[idx] - sub[idx];
    }
}

#[inline(always)]
pub(crate) fn psqt_add2_sub2_into(
    base: &[i32; PSQT_BUCKETS],
    add0: &[i32],
    add1: &[i32],
    sub0: &[i32],
    sub1: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    debug_assert_eq!(add0.len(), PSQT_BUCKETS);
    debug_assert_eq!(add1.len(), PSQT_BUCKETS);
    debug_assert_eq!(sub0.len(), PSQT_BUCKETS);
    debug_assert_eq!(sub1.len(), PSQT_BUCKETS);

    #[cfg(target_arch = "x86_64")]
    if can_vectorize_i32_6(
        base.as_ptr(),
        add0.as_ptr(),
        add1.as_ptr(),
        sub0.as_ptr(),
        sub1.as_ptr(),
        out.as_ptr(),
    ) {
        unsafe {
            psqt_add2_sub2_into_avx2(base, add0, add1, sub0, sub1, out);
        }
        return;
    }

    for idx in 0..PSQT_BUCKETS {
        out[idx] = base[idx] + add0[idx] + add1[idx] - sub0[idx] - sub1[idx];
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn is_32_byte_aligned<T>(ptr: *const T) -> bool {
    (ptr as usize & 31) == 0
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i16(a: *const i16, b: *const i16, len: usize) -> bool {
    len % 16 == 0 && is_32_byte_aligned(a) && is_32_byte_aligned(b)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i16_3(a: *const i16, b: *const i16, c: *const i16, len: usize) -> bool {
    len % 16 == 0 && is_32_byte_aligned(a) && is_32_byte_aligned(b) && is_32_byte_aligned(c)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i16_4(
    a: *const i16,
    b: *const i16,
    c: *const i16,
    d: *const i16,
    len: usize,
) -> bool {
    len % 16 == 0
        && is_32_byte_aligned(a)
        && is_32_byte_aligned(b)
        && is_32_byte_aligned(c)
        && is_32_byte_aligned(d)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i16_5(
    a: *const i16,
    b: *const i16,
    c: *const i16,
    d: *const i16,
    e: *const i16,
    len: usize,
) -> bool {
    can_vectorize_i16_4(a, b, c, d, len) && is_32_byte_aligned(e)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i16_6(
    a: *const i16,
    b: *const i16,
    c: *const i16,
    d: *const i16,
    e: *const i16,
    f: *const i16,
    len: usize,
) -> bool {
    can_vectorize_i16_5(a, b, c, d, e, len) && is_32_byte_aligned(f)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i32(a: *const i32, b: *const i32) -> bool {
    is_32_byte_aligned(a) && is_32_byte_aligned(b)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i32_3(a: *const i32, b: *const i32, c: *const i32) -> bool {
    can_vectorize_i32(a, b) && is_32_byte_aligned(c)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i32_4(a: *const i32, b: *const i32, c: *const i32, d: *const i32) -> bool {
    can_vectorize_i32_3(a, b, c) && is_32_byte_aligned(d)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i32_5(
    a: *const i32,
    b: *const i32,
    c: *const i32,
    d: *const i32,
    e: *const i32,
) -> bool {
    can_vectorize_i32_4(a, b, c, d) && is_32_byte_aligned(e)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn can_vectorize_i32_6(
    a: *const i32,
    b: *const i32,
    c: *const i32,
    d: *const i32,
    e: *const i32,
    f: *const i32,
) -> bool {
    can_vectorize_i32_5(a, b, c, d, e) && is_32_byte_aligned(f)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accum_add_avx2<const N: usize>(acc: &mut [i16; N], add: &[i16]) {
    for idx in (0..N).step_by(16) {
        unsafe {
            let sum = _mm256_add_epi16(
                _mm256_load_si256(acc.as_ptr().add(idx).cast::<__m256i>()),
                _mm256_load_si256(add.as_ptr().add(idx).cast::<__m256i>()),
            );
            _mm256_store_si256(acc.as_mut_ptr().add(idx).cast::<__m256i>(), sum);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accum_sub_avx2<const N: usize>(acc: &mut [i16; N], sub: &[i16]) {
    for idx in (0..N).step_by(16) {
        unsafe {
            let sum = _mm256_sub_epi16(
                _mm256_load_si256(acc.as_ptr().add(idx).cast::<__m256i>()),
                _mm256_load_si256(sub.as_ptr().add(idx).cast::<__m256i>()),
            );
            _mm256_store_si256(acc.as_mut_ptr().add(idx).cast::<__m256i>(), sum);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accum_add_sub_avx2<const N: usize>(acc: &mut [i16; N], add: &[i16], sub: &[i16]) {
    for idx in (0..N).step_by(16) {
        unsafe {
            let base = _mm256_load_si256(acc.as_ptr().add(idx).cast::<__m256i>());
            let sum = _mm256_add_epi16(
                base,
                _mm256_sub_epi16(
                    _mm256_load_si256(add.as_ptr().add(idx).cast::<__m256i>()),
                    _mm256_load_si256(sub.as_ptr().add(idx).cast::<__m256i>()),
                ),
            );
            _mm256_store_si256(acc.as_mut_ptr().add(idx).cast::<__m256i>(), sum);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accum_add1_sub1_into_avx2<const N: usize>(
    base: &[i16; N],
    add: &[i16],
    sub: &[i16],
    out: &mut [i16; N],
) {
    for idx in (0..N).step_by(16) {
        unsafe {
            let value = _mm256_add_epi16(
                _mm256_load_si256(base.as_ptr().add(idx).cast::<__m256i>()),
                _mm256_sub_epi16(
                    _mm256_load_si256(add.as_ptr().add(idx).cast::<__m256i>()),
                    _mm256_load_si256(sub.as_ptr().add(idx).cast::<__m256i>()),
                ),
            );
            _mm256_store_si256(out.as_mut_ptr().add(idx).cast::<__m256i>(), value);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accum_add1_sub2_into_avx2<const N: usize>(
    base: &[i16; N],
    add: &[i16],
    sub0: &[i16],
    sub1: &[i16],
    out: &mut [i16; N],
) {
    for idx in (0..N).step_by(16) {
        unsafe {
            let value = _mm256_sub_epi16(
                _mm256_add_epi16(
                    _mm256_load_si256(base.as_ptr().add(idx).cast::<__m256i>()),
                    _mm256_load_si256(add.as_ptr().add(idx).cast::<__m256i>()),
                ),
                _mm256_add_epi16(
                    _mm256_load_si256(sub0.as_ptr().add(idx).cast::<__m256i>()),
                    _mm256_load_si256(sub1.as_ptr().add(idx).cast::<__m256i>()),
                ),
            );
            _mm256_store_si256(out.as_mut_ptr().add(idx).cast::<__m256i>(), value);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accum_add2_sub1_into_avx2<const N: usize>(
    base: &[i16; N],
    add0: &[i16],
    add1: &[i16],
    sub: &[i16],
    out: &mut [i16; N],
) {
    for idx in (0..N).step_by(16) {
        unsafe {
            let value = _mm256_add_epi16(
                _mm256_add_epi16(
                    _mm256_load_si256(base.as_ptr().add(idx).cast::<__m256i>()),
                    _mm256_load_si256(add0.as_ptr().add(idx).cast::<__m256i>()),
                ),
                _mm256_sub_epi16(
                    _mm256_load_si256(add1.as_ptr().add(idx).cast::<__m256i>()),
                    _mm256_load_si256(sub.as_ptr().add(idx).cast::<__m256i>()),
                ),
            );
            _mm256_store_si256(out.as_mut_ptr().add(idx).cast::<__m256i>(), value);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accum_add2_sub2_into_avx2<const N: usize>(
    base: &[i16; N],
    add0: &[i16],
    add1: &[i16],
    sub0: &[i16],
    sub1: &[i16],
    out: &mut [i16; N],
) {
    for idx in (0..N).step_by(16) {
        unsafe {
            let value = _mm256_add_epi16(
                _mm256_load_si256(base.as_ptr().add(idx).cast::<__m256i>()),
                _mm256_sub_epi16(
                    _mm256_add_epi16(
                        _mm256_load_si256(add0.as_ptr().add(idx).cast::<__m256i>()),
                        _mm256_load_si256(add1.as_ptr().add(idx).cast::<__m256i>()),
                    ),
                    _mm256_add_epi16(
                        _mm256_load_si256(sub0.as_ptr().add(idx).cast::<__m256i>()),
                        _mm256_load_si256(sub1.as_ptr().add(idx).cast::<__m256i>()),
                    ),
                ),
            );
            _mm256_store_si256(out.as_mut_ptr().add(idx).cast::<__m256i>(), value);
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn psqt_add_avx2(psqt: &mut [i32; PSQT_BUCKETS], add: &[i32]) {
    unsafe {
        let value = _mm256_add_epi32(
            _mm256_load_si256(psqt.as_ptr().cast::<__m256i>()),
            _mm256_load_si256(add.as_ptr().cast::<__m256i>()),
        );
        _mm256_store_si256(psqt.as_mut_ptr().cast::<__m256i>(), value);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn psqt_sub_avx2(psqt: &mut [i32; PSQT_BUCKETS], sub: &[i32]) {
    unsafe {
        let value = _mm256_sub_epi32(
            _mm256_load_si256(psqt.as_ptr().cast::<__m256i>()),
            _mm256_load_si256(sub.as_ptr().cast::<__m256i>()),
        );
        _mm256_store_si256(psqt.as_mut_ptr().cast::<__m256i>(), value);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn psqt_add_sub_avx2(psqt: &mut [i32; PSQT_BUCKETS], add: &[i32], sub: &[i32]) {
    unsafe {
        let value = _mm256_add_epi32(
            _mm256_load_si256(psqt.as_ptr().cast::<__m256i>()),
            _mm256_sub_epi32(
                _mm256_load_si256(add.as_ptr().cast::<__m256i>()),
                _mm256_load_si256(sub.as_ptr().cast::<__m256i>()),
            ),
        );
        _mm256_store_si256(psqt.as_mut_ptr().cast::<__m256i>(), value);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn psqt_add1_sub1_into_avx2(
    base: &[i32; PSQT_BUCKETS],
    add: &[i32],
    sub: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    unsafe {
        let value = _mm256_add_epi32(
            _mm256_load_si256(base.as_ptr().cast::<__m256i>()),
            _mm256_sub_epi32(
                _mm256_load_si256(add.as_ptr().cast::<__m256i>()),
                _mm256_load_si256(sub.as_ptr().cast::<__m256i>()),
            ),
        );
        _mm256_store_si256(out.as_mut_ptr().cast::<__m256i>(), value);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn psqt_add1_sub2_into_avx2(
    base: &[i32; PSQT_BUCKETS],
    add: &[i32],
    sub0: &[i32],
    sub1: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    unsafe {
        let value = _mm256_sub_epi32(
            _mm256_add_epi32(
                _mm256_load_si256(base.as_ptr().cast::<__m256i>()),
                _mm256_load_si256(add.as_ptr().cast::<__m256i>()),
            ),
            _mm256_add_epi32(
                _mm256_load_si256(sub0.as_ptr().cast::<__m256i>()),
                _mm256_load_si256(sub1.as_ptr().cast::<__m256i>()),
            ),
        );
        _mm256_store_si256(out.as_mut_ptr().cast::<__m256i>(), value);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn psqt_add2_sub1_into_avx2(
    base: &[i32; PSQT_BUCKETS],
    add0: &[i32],
    add1: &[i32],
    sub: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    unsafe {
        let value = _mm256_add_epi32(
            _mm256_add_epi32(
                _mm256_load_si256(base.as_ptr().cast::<__m256i>()),
                _mm256_load_si256(add0.as_ptr().cast::<__m256i>()),
            ),
            _mm256_sub_epi32(
                _mm256_load_si256(add1.as_ptr().cast::<__m256i>()),
                _mm256_load_si256(sub.as_ptr().cast::<__m256i>()),
            ),
        );
        _mm256_store_si256(out.as_mut_ptr().cast::<__m256i>(), value);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn psqt_add2_sub2_into_avx2(
    base: &[i32; PSQT_BUCKETS],
    add0: &[i32],
    add1: &[i32],
    sub0: &[i32],
    sub1: &[i32],
    out: &mut [i32; PSQT_BUCKETS],
) {
    unsafe {
        let value = _mm256_add_epi32(
            _mm256_load_si256(base.as_ptr().cast::<__m256i>()),
            _mm256_sub_epi32(
                _mm256_add_epi32(
                    _mm256_load_si256(add0.as_ptr().cast::<__m256i>()),
                    _mm256_load_si256(add1.as_ptr().cast::<__m256i>()),
                ),
                _mm256_add_epi32(
                    _mm256_load_si256(sub0.as_ptr().cast::<__m256i>()),
                    _mm256_load_si256(sub1.as_ptr().cast::<__m256i>()),
                ),
            ),
        );
        _mm256_store_si256(out.as_mut_ptr().cast::<__m256i>(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::{accum_add_sub, accum_add1_sub2_into, psqt_add2_sub2_into};
    use crate::aligned::CacheAligned;

    #[test]
    fn vector_accum_update_matches_scalar_reference() {
        let base = CacheAligned::new(std::array::from_fn(|i| (i as i16) * 3 - 77));
        let add: CacheAligned<[i16; 128]> =
            CacheAligned::new(std::array::from_fn(|i| (i as i16) * 5 - 11));
        let sub0: CacheAligned<[i16; 128]> =
            CacheAligned::new(std::array::from_fn(|i| (i as i16) * 7 - 29));
        let sub1: CacheAligned<[i16; 128]> =
            CacheAligned::new(std::array::from_fn(|i| 101 - i as i16));
        let mut out = CacheAligned::new([0i16; 128]);
        let mut scalar = [0i16; 128];

        accum_add1_sub2_into(&base, &add[..], &sub0[..], &sub1[..], &mut out);
        for i in 0..128 {
            scalar[i] = base[i]
                .wrapping_add(add[i])
                .wrapping_sub(sub0[i])
                .wrapping_sub(sub1[i]);
        }
        assert_eq!(*out, scalar);

        let mut inplace = base.clone();
        let mut inplace_scalar = base.0;
        accum_add_sub(&mut inplace, &add[..], &sub0[..]);
        for i in 0..128 {
            inplace_scalar[i] = inplace_scalar[i].wrapping_add(add[i]).wrapping_sub(sub0[i]);
        }
        assert_eq!(*inplace, inplace_scalar);
    }

    #[test]
    fn vector_psqt_update_matches_scalar_reference() {
        let base = CacheAligned::new([1, 2, 3, 4, 5, 6, 7, 8]);
        let add0 = CacheAligned::new([3, 1, -2, 5, 8, -1, 4, 7]);
        let add1 = CacheAligned::new([2, 6, 1, -3, 4, 0, 5, -2]);
        let sub0 = CacheAligned::new([-1, 2, 4, 3, -5, 7, 1, 9]);
        let sub1 = CacheAligned::new([8, -4, 3, 2, 1, 5, -6, 0]);
        let mut out = CacheAligned::new([0i32; 8]);
        let mut scalar = [0i32; 8];

        psqt_add2_sub2_into(&base, &add0[..], &add1[..], &sub0[..], &sub1[..], &mut out);
        for i in 0..8 {
            scalar[i] = base[i] + add0[i] + add1[i] - sub0[i] - sub1[i];
        }
        assert_eq!(*out, scalar);
    }
}
