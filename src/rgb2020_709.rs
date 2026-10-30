fn linear_bt2020_to_rgb8(r_sdr: &[f32], g_sdr: &[f32], b_sdr: &[f32]) -> Vec<u8> {
    fn linear_to_srgb_u8(v: f32) -> u8 {
        let v = v.clamp(0.0, 1.0);

        let srgb = if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        };

        (srgb * 255.0 + 0.5) as u8
    }

    let mut rgb8 = Vec::with_capacity(r_sdr.len() * 3);

    for i in 0..r_sdr.len() {
        let r = r_sdr[i];
        let g = g_sdr[i];
        let b = b_sdr[i];

        // BT.2020 linear -> BT.709 linear
        let r709 = 1.6605 * r - 0.5876 * g - 0.0728 * b;
        let g709 = -0.1246 * r + 1.1329 * g - 0.0083 * b;
        let b709 = -0.0182 * r - 0.1006 * g + 1.1187 * b;

        rgb8.push(linear_to_srgb_u8(r709));
        rgb8.push(linear_to_srgb_u8(g709));
        rgb8.push(linear_to_srgb_u8(b709));
    }

    rgb8
}
