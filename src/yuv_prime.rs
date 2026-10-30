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

pub fn xyz_to_yuvprime_wide16(
    x_in: &[f32],
    y_in: &[f32],
    z_in: &[f32],
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    const LANES: usize = 16;

    let n = x_in.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let mut y_out = vec![0.0f32; n];
    let mut u_out = vec![0.0f32; n];
    let mut v_out = vec![0.0f32; n];

    let four = f32x16::splat(4.0f32);
    let nine = f32x16::splat(9.0f32);
    let fifteen = f32x16::splat(15.0f32);
    let three = f32x16::splat(3.0f32);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let vx = f32x16::new(load_n!(x_in, i, LANES));
        let vy = f32x16::new(load_n!(y_in, i, LANES));
        let vz = f32x16::new(load_n!(z_in, i, LANES));

        let denom = vx + fifteen * vy + three * vz;

        let vu = (four * vx) / denom;
        let vv = (nine * vy) / denom;

        store_n!(vy, y_out, i, LANES); // Y stays Y
        store_n!(vu, u_out, i, LANES);
        store_n!(vv, v_out, i, LANES);
    }

    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            let x = x_in[i];
            let y = y_in[i];
            let z = z_in[i];

            let denom = x + 15.0 * y + 3.0 * z;

            y_out[i] = y;
            u_out[i] = 4.0 * x / denom;
            v_out[i] = 9.0 * y / denom;
        }
    }

    (y_out, u_out, v_out)
}

pub fn yuvprime_to_xyz_wide16(
    y_in: &[f32],
    u_in: &[f32],
    v_in: &[f32],
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    const LANES: usize = 16;

    let n = y_in.len();
    let simd_chunks = n / LANES;
    let remainder = n % LANES;

    let mut x_out = vec![0.0f32; n];
    let mut y_out = vec![0.0f32; n];
    let mut z_out = vec![0.0f32; n];

    let nine = f32x16::splat(9.0f32);
    let four = f32x16::splat(4.0f32);
    let twelve = f32x16::splat(12.0f32);
    let three = f32x16::splat(3.0f32);
    let twenty = f32x16::splat(20.0f32);

    for chunk in 0..simd_chunks {
        let i = chunk * LANES;

        let vy = f32x16::new(load_n!(y_in, i, LANES));
        let vu = f32x16::new(load_n!(u_in, i, LANES));
        let vv = f32x16::new(load_n!(v_in, i, LANES));

        let denom = four * vv;

        let vx = (nine * vu * vy) / denom;
        let vz = ((twelve - three * vu - twenty * vv) * vy) / denom;

        store_n!(vx, x_out, i, LANES);
        store_n!(vy, y_out, i, LANES);
        store_n!(vz, z_out, i, LANES);
    }

    if remainder > 0 {
        let start = simd_chunks * LANES;
        for i in start..n {
            let y = y_in[i];
            let u = u_in[i];
            let v = v_in[i];

            let denom = 4.0 * v;

            x_out[i] = (9.0 * u * y) / denom;
            y_out[i] = y;
            z_out[i] = ((12.0 - 3.0 * u - 20.0 * v) * y) / denom;
        }
    }

    (x_out, y_out, z_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xyz_yuvprime_roundtrip() {
        // 10 test XYZ values (non-zero Y so v' is valid)
        let x = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let y = [0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50];
        let z = [0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1];

        // Convert XYZ → Y u' v'
        let (yy, uu, vv) = xyz_to_yuvprime_wide16(&x, &y, &z);

        // Convert Y u' v' → XYZ
        let (xx, yy2, zz) = yuvprime_to_xyz_wide16(&yy, &uu, &vv);

        // Check error tolerance
        for i in 0..x.len() {
            let dx = (xx[i] - x[i]).abs();
            let dy = (yy2[i] - y[i]).abs();
            let dz = (zz[i] - z[i]).abs();

            assert!(dx < 1e-5, "X mismatch at {}: {} vs {}", i, xx[i], x[i]);
            assert!(dy < 1e-5, "Y mismatch at {}: {} vs {}", i, yy2[i], y[i]);
            assert!(dz < 1e-5, "Z mismatch at {}: {} vs {}", i, zz[i], z[i]);
        }
    }
}
