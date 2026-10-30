use std::time::Instant;
use wide::f32x16;

macro_rules! load_n {
    ($slice:expr, $i:expr, $lanes:expr) => {{
        let mut arr = [0.0f32; $lanes];
        let mut j = 0;
        while j < $lanes {
            arr[j] = $slice[$i + j];
            j += 1;
        }
        arr
    }};
}

macro_rules! store_n {
    ($vec:expr, $out:expr, $i:expr, $lanes:expr) => {{
        let arr = $vec.to_array();
        let mut j = 0;
        while j < $lanes {
            $out[$i + j] = arr[j];
            j += 1;
        }
    }};
}

pub fn mul_vec_scalar_wide16(v: &mut [f32], k: f32) {
    const LANES: usize = 16;

    let n = v.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let vk = f32x16::splat(k);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let vv = f32x16::new(load_n!(v, i, LANES));
        let vr = vv * vk;

        store_n!(vr, v, i, LANES);
    }

    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            v[i] *= k;
        }
    }
}

pub fn mul_vec_scalar(v: &mut [f32], k: f32) {
    for x in v.iter_mut() {
        *x *= k;
    }
}

pub(crate) fn mul_vec_scalar_new_wide16(b_in: &[f32], k: f32) -> Vec<f32> {
    const LANES: usize = 16;

    let n = b_in.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let mut out = vec![0.0f32; n];
    let vk = f32x16::splat(k);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let vb = f32x16::new(load_n!(b_in, i, LANES));
        let va = vb * vk;

        store_n!(va, out.as_mut_slice(), i, LANES);
    }

    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            out[i] = b_in[i] * k;
        }
    }

    out
}
