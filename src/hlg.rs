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

pub(crate) fn hlg_eotf_wide16(e_in: &[f32]) -> Vec<f32> {
    const LANES: usize = 16;

    let n = e_in.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let mut out = vec![0.0f32; n];

    // HLG constants
    let a = f32x16::splat(0.17883277f32);
    let b = f32x16::splat(1.0f32 - 4.0f32 * 0.17883277f32);
    let c = f32x16::splat(0.5f32 - 0.17883277f32 * (4.0f32 * 0.17883277f32).ln());

    let half = f32x16::splat(0.5f32);
    let three = f32x16::splat(3.0f32);
    let twelve = f32x16::splat(12.0f32);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let ve = f32x16::new(load_n!(e_in, i, LANES));

        // Compute both branches
        let l_low = (ve * ve) / three;
        let l_high = ((ve - c) / a).exp();
        let l_high = (l_high + b) / twelve;

        // Manual mask: element-wise compare
        let mut mask_arr = [0.0f32; LANES];
        let ve_arr = ve.to_array();
        for j in 0..LANES {
            mask_arr[j] = if ve_arr[j] <= 0.5f32 { 1.0 } else { 0.0 };
        }
        let mask = f32x16::new(mask_arr);

        // Blend: mask * low + (1-mask) * high
        let one = f32x16::splat(1.0f32);
        let vl = mask * l_low + (one - mask) * l_high;

        store_n!(vl, out, i, LANES);
    }

    // Scalar tail
    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            let e = e_in[i];
            if e <= 0.5 {
                out[i] = (e * e) / 3.0;
            } else {
                let a = 0.17883277f32;
                let b = 1.0f32 - 4.0f32 * a;
                let c = 0.5f32 - a * (4.0f32 * a).ln();
                let l = ((e - c) / a).exp();
                out[i] = (l + b) / 12.0f32;
            }
        }
    }

    out
}

pub(crate) fn hlg_eotf(e: f32) -> f32 {
    const A: f32 = 0.17883277;
    const B: f32 = 1.0 - 4.0 * A;
    let c = 0.5 - A * (4.0 * A).ln();

    if e <= 0.5 {
        (e * e) / 3.0
    } else {
        (f32::exp((e - c) / A) + B) / 12.0
    }
}

pub(crate) fn hlg_oetf(l: f32) -> f32 {
    const A: f32 = 0.17883277;
    const B: f32 = 1.0 - 4.0 * A;
    let c = 0.5 - A * (4.0 * A).ln();

    if l <= 1.0 / 12.0 {
        (3.0 * l).sqrt()
    } else {
        A * (12.0 * l - B).ln() + c
    }
}

pub(crate) fn hlg_oetf_wide16(l_in: &[f32]) -> Vec<f32> {
    const LANES: usize = 16;

    let n = l_in.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;
    let mut out = vec![0.0f32; n];

    let a = f32x16::splat(0.17883277f32);
    let b = f32x16::splat(1.0f32 - 4.0f32 * 0.17883277f32);
    let c = f32x16::splat(0.5f32 - 0.17883277f32 * (4.0f32 * 0.17883277f32).ln());
    let three = f32x16::splat(3.0f32);
    let twelve = f32x16::splat(12.0f32);
    let one = f32x16::splat(1.0f32);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;
        let vl = f32x16::new(load_n!(l_in, i, LANES));

        let mut mask_arr = [0.0f32; LANES];
        let vl_arr = vl.to_array();
        for j in 0..LANES {
            mask_arr[j] = if vl_arr[j] <= 1.0f32 / 12.0f32 { 1.0 } else { 0.0 };
        }
        let mask = f32x16::new(mask_arr);

        // Keep the logarithm's input positive in lanes using the low branch.
        let high_input = mask * one + (one - mask) * (twelve * vl - b);
        let e_low = (three * vl).sqrt();
        let e_high = a * high_input.ln() + c;
        let ve = mask * e_low + (one - mask) * e_high;

        store_n!(ve, out, i, LANES);
    }

    // Scalar tail
    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            let l = l_in[i];
            if l <= 1.0f32 / 12.0f32 {
                out[i] = (3.0f32 * l).sqrt();
            } else {
                let a = 0.17883277f32;
                let b = 1.0f32 - 4.0f32 * a;
                let c = 0.5f32 - a * (4.0f32 * a).ln();
                out[i] = a * (12.0f32 * l - b).ln() + c;
            }
        }
    }

    out
}

/*

pub(crate) fn apply_hlg_eotf_iter(input: &[f32]) -> Vec<f32> {
    input.iter().map(|&e| hlg_eotf(e)).collect()
}

//highly parallel
use rayon::prelude::*;
pub (crate) fn apply_hlg_eotf_parallel(input: &[f32]) -> Vec<f32> {
    input.par_iter()
        .map(|&e| hlg_eotf(e))
        .collect()
}

 */

#[cfg(test)]
mod tests {
    use super::{hlg_eotf, hlg_eotf_wide16, hlg_oetf, hlg_oetf_wide16};

    #[test]
    fn eotf_then_oetf_has_small_round_trip_error() {
        const TOLERANCE: f32 = 2.0e-6;

        let inputs: Vec<f32> = (0..=1024).map(|i| i as f32 / 1024.0).collect();

        let scalar_max_error = inputs
            .iter()
            .map(|&e| (hlg_oetf(hlg_eotf(e)) - e).abs())
            .fold(0.0f32, f32::max);
        assert!(
            scalar_max_error <= TOLERANCE,
            "scalar EOTF→OETF max error {scalar_max_error} exceeds {TOLERANCE}"
        );

        let linear = hlg_eotf_wide16(&inputs);
        let round_trip = hlg_oetf_wide16(&linear);
        let wide_max_error = inputs
            .iter()
            .zip(round_trip)
            .map(|(&expected, actual)| (actual - expected).abs())
            .fold(0.0f32, f32::max);
        assert!(
            wide_max_error <= TOLERANCE,
            "wide16 EOTF→OETF max error {wide_max_error} exceeds {TOLERANCE}"
        );
    }
}
