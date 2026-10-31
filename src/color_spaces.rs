//use std::os::raw::c_uint;
//use wide::f32x8;
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

pub fn bt2020_yuv_to_rgb_wide16(
    y: &[f32],
    cb: &[f32],
    cr: &[f32],
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    const LANES: usize = 16;
    let n = y.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let mut r_out = vec![0.0f32; n];
    let mut g_out = vec![0.0f32; n];
    let mut b_out = vec![0.0f32; n];

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let vy = f32x16::new(load_n!(y, i, LANES));
        let vcb = f32x16::new(load_n!(cb, i, LANES));
        let vcr = f32x16::new(load_n!(cr, i, LANES));

        let vr = vy + vcr * f32x16::splat(1.4746);
        let vg = vy - vcb * f32x16::splat(0.16455) - vcr * f32x16::splat(0.57135);
        let vb = vy + vcb * f32x16::splat(1.8814);

        store_n!(vr, r_out, i, LANES);
        store_n!(vg, g_out, i, LANES);
        store_n!(vb, b_out, i, LANES);
    }

    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            let yv = y[i];
            let cbv = cb[i];
            let crv = cr[i];

            r_out[i] = yv + 1.4746 * crv;
            g_out[i] = yv - 0.16455 * cbv - 0.57135 * crv;
            b_out[i] = yv + 1.8814 * cbv;
        }
    }

    (r_out, g_out, b_out)
}

#[inline]
pub(crate) fn bt2020_yuv_to_rgb(y: f32, cb: f32, cr: f32) -> (f32, f32, f32) {
    let r = y + 1.4746 * cr;
    let g = y - 0.16455 * cb - 0.57135 * cr;
    let b = y + 1.8814 * cb;
    (r, g, b)
}

pub(crate) fn rgb_to_xyz_wide16(
    r_in: &[f32],
    g_in: &[f32],
    b_in: &[f32],
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    const LANES: usize = 16;

    let n = r_in.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let mut x_out = vec![0.0f32; n];
    let mut y_out = vec![0.0f32; n];
    let mut z_out = vec![0.0f32; n];

    let m00 = f32x16::splat(0.636958048f32);
    let m01 = f32x16::splat(0.144616903f32);
    let m02 = f32x16::splat(0.168880975f32);

    let m10 = f32x16::splat(0.262700212f32);
    let m11 = f32x16::splat(0.677998071f32);
    let m12 = f32x16::splat(0.059301717f32);

    let m20 = f32x16::splat(0.000000000f32);
    let m21 = f32x16::splat(0.028072693f32);
    let m22 = f32x16::splat(1.060985058f32);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let vr = f32x16::new(load_n!(r_in, i, LANES));
        let vg = f32x16::new(load_n!(g_in, i, LANES));
        let vb = f32x16::new(load_n!(b_in, i, LANES));

        let vx = vr * m00 + vg * m01 + vb * m02;
        let vy = vr * m10 + vg * m11 + vb * m12;
        let vz = vr * m20 + vg * m21 + vb * m22;

        store_n!(vx, x_out, i, LANES);
        store_n!(vy, y_out, i, LANES);
        store_n!(vz, z_out, i, LANES);
    }

    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            let r = r_in[i];
            let g = g_in[i];
            let b = b_in[i];

            x_out[i] = 0.636958048 * r + 0.144616903 * g + 0.168880975 * b;
            y_out[i] = 0.262700212 * r + 0.677998071 * g + 0.059301717 * b;
            z_out[i] = 0.000000000 * r + 0.028072693 * g + 1.060985058 * b;
        }
    }

    (x_out, y_out, z_out)
}

pub(crate) fn xyz_to_rgb_wide16(
    x_in: &[f32],
    y_in: &[f32],
    z_in: &[f32],
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    const LANES: usize = 16;

    let n = x_in.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let mut r_out = vec![0.0f32; n];
    let mut g_out = vec![0.0f32; n];
    let mut b_out = vec![0.0f32; n];

    let m00 = f32x16::splat(1.716651188f32);
    let m01 = f32x16::splat(-0.355670783f32);
    let m02 = f32x16::splat(-0.253366281f32);

    let m10 = f32x16::splat(-0.666684351f32);
    let m11 = f32x16::splat(1.616481236f32);
    let m12 = f32x16::splat(0.015768546f32);

    let m20 = f32x16::splat(0.017639857f32);
    let m21 = f32x16::splat(-0.042770613f32);
    let m22 = f32x16::splat(0.942103121f32);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let vx = f32x16::new(load_n!(x_in, i, LANES));
        let vy = f32x16::new(load_n!(y_in, i, LANES));
        let vz = f32x16::new(load_n!(z_in, i, LANES));

        let vr = vx * m00 + vy * m01 + vz * m02;
        let vg = vx * m10 + vy * m11 + vz * m12;
        let vb = vx * m20 + vy * m21 + vz * m22;

        store_n!(vr, r_out, i, LANES);
        store_n!(vg, g_out, i, LANES);
        store_n!(vb, b_out, i, LANES);
    }

    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            let x = x_in[i];
            let y = y_in[i];
            let z = z_in[i];

            r_out[i] = 1.716651188 * x - 0.355670783 * y - 0.253366281 * z;
            g_out[i] = -0.666684351 * x + 1.616481236 * y + 0.015768546 * z;
            b_out[i] = 0.017639857 * x - 0.042770613 * y + 0.942103121 * z;
        }
    }

    (r_out, g_out, b_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_xyz_roundtrip() {
        // 10 test RGB values
        let r = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0];
        let g = [0.0, 0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 1.0];
        let b = [0.0, 0.2, 0.4, 0.6, 0.8, 0.1, 0.3, 0.5, 0.7, 1.0];

        // Convert RGB → XYZ
        let (x, y, z) = rgb_to_xyz_wide16(&r, &g, &b);

        // Convert XYZ → RGB
        let (rr, gg, bb) = xyz_to_rgb_wide16(&x, &y, &z);

        // Check error tolerance
        for i in 0..r.len() {
            let dr = (rr[i] - r[i]).abs();
            let dg = (gg[i] - g[i]).abs();
            let db = (bb[i] - b[i]).abs();

            assert!(dr < 1e-5, "R mismatch at {}: {} vs {}", i, rr[i], r[i]);
            assert!(dg < 1e-5, "G mismatch at {}: {} vs {}", i, gg[i], g[i]);
            assert!(db < 1e-5, "B mismatch at {}: {} vs {}", i, bb[i], b[i]);
        }
    }
}
