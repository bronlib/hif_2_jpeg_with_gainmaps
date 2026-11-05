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
