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

/// Applies the SMPTE ST 2084 (PQ) EOTF to encoded values in [0, 1].
///
/// The output is linear luminance normalized to 10,000 cd/m², so `1.0`
/// represents 10,000 cd/m². Inputs are clamped to [0, 1].
pub(crate) fn pq_eotf_wide16(e_in: &[f32]) -> Vec<f32> {
    const LANES: usize = 16;
    const M1: f32 = 2610.0 / 16384.0;
    const M2: f32 = 2523.0 / 32.0;
    const C1: f32 = 3424.0 / 4096.0;
    const C2: f32 = 2413.0 / 128.0;
    const C3: f32 = 2392.0 / 128.0;

    let n = e_in.len();
    let simd_chunks = n / LANES;
    let mut out = vec![0.0f32; n];
    let m1_inv = f32x16::splat(1.0 / M1);
    let m2 = f32x16::splat(M2);
    let c1 = f32x16::splat(C1);
    let c2 = f32x16::splat(C2);
    let c3 = f32x16::splat(C3);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;
        let mut values = load_n!(e_in, i, LANES);
        for value in &mut values {
            *value = value.clamp(0.0, 1.0);
        }

        let ve = f32x16::new(values);
        let p = ve.powf_simd(m2);
        let numerator = (p - c1).max(f32x16::splat(0.0));
        let denominator = c2 - c3 * p;
        let ratio = numerator / denominator;
        let linear = ratio.powf_simd(m1_inv);

        store_n!(linear, out, i, LANES);
    }

    for i in (simd_chunks * LANES)..n {
        let e = e_in[i].clamp(0.0, 1.0);
        let p = e.powf(M2);
        let numerator = (p - C1).max(0.0);
        let denominator = C2 - C3 * p;
        out[i] = (numerator / denominator).powf(1.0 / M1);
    }

    out
}
