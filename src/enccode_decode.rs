
use zenjpeg::ultrahdr::{
    UhdrColorGamut, UhdrColorTransfer, UhdrPixelFormat, UhdrRawImage,
};


pub fn to_raw(width: u32, height: u32, r: &[f32], g: &[f32], b: &[f32]) -> UhdrRawImage {
    let mut rgb8 = Vec::with_capacity(r.len() * 3);

    for ((&r, &g), &b) in r.iter().zip(g.iter()).zip(b.iter()) {
        rgb8.push((r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
        rgb8.push((g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
        rgb8.push((b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
    }

    UhdrRawImage::from_data(
        width,
        height,
        UhdrPixelFormat::Rgb8,
        UhdrColorGamut::Bt2020,
        UhdrColorTransfer::Linear,
        rgb8,
    )
    .unwrap()
}
