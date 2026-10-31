use ultrahdr_core::{ColorTransfer, RawImage};
use zenjpeg::ultrahdr::{UhdrColorGamut, UhdrPixelFormat};

pub(crate) fn bt2020_to_bt709(image: &RawImage) -> RawImage {
    assert_eq!(image.format, UhdrPixelFormat::Rgba32F);

    let mut output = image.clone();

    for pixel in output.data.chunks_exact_mut(16) {
        let r = f32::from_ne_bytes(pixel[0..4].try_into().unwrap());
        let g = f32::from_ne_bytes(pixel[4..8].try_into().unwrap());
        let b = f32::from_ne_bytes(pixel[8..12].try_into().unwrap());

        let r709 = 1.6605 * r - 0.5876 * g - 0.0728 * b;
        let g709 = -0.1246 * r + 1.1329 * g - 0.0083 * b;
        let b709 = -0.0182 * r - 0.1006 * g + 1.1187 * b;

        pixel[0..4].copy_from_slice(&r709.to_ne_bytes());
        pixel[4..8].copy_from_slice(&g709.to_ne_bytes());
        pixel[8..12].copy_from_slice(&b709.to_ne_bytes());
    }

    output.gamut = UhdrColorGamut::Bt709;
    output.transfer = ColorTransfer::Linear;

    output
}

pub(crate) fn bt709_to_bt2020(image: &RawImage) -> RawImage {
    assert_eq!(image.format, UhdrPixelFormat::Rgba32F);

    fn decode_bt709(v: f32) -> f32 {
        if v < 0.081 {
            v / 4.5
        } else {
            ((v + 0.099) / 1.099).powf(1.0 / 0.45)
        }
    }

    fn encode_bt709(v: f32) -> f32 {
        if v < 0.018 {
            4.5 * v
        } else {
            1.099 * v.powf(0.45) - 0.099
        }
    }

    let mut output = image.clone();

    for y in 0..image.height as usize {
        let row_start = y * image.stride as usize;

        for x in 0..image.width as usize {
            let pixel_start = row_start + x * 16; // 4 × f32
            let read = |channel: usize| {
                let start = pixel_start + channel * 4;
                f32::from_ne_bytes(image.data[start..start + 4].try_into().unwrap())
            };

            let [r, g, b] = [read(0), read(1), read(2)].map(decode_bt709);

            let converted = [
                0.627_403_9 * r + 0.329_283_0 * g + 0.043_313_1 * b,
                0.069_097_3 * r + 0.919_540_4 * g + 0.011_362_3 * b,
                0.016_391_4 * r + 0.088_013_3 * g + 0.895_595_3 * b,
            ]
                .map(encode_bt709);

            for (channel, value) in converted.into_iter().enumerate() {
                let start = pixel_start + channel * 4;
                output.data[start..start + 4].copy_from_slice(&value.to_ne_bytes());
            }
        }
    }

    output.gamut = UhdrColorGamut::Bt2020;
    output
}
