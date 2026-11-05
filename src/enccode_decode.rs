


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

/*
pub(crate) fn encode_zen(
    to_file: &str,
    width: usize,
    height: usize,
    r_hdr: &[f32],
    g_hdr: &[f32],
    b_hdr: &[f32],
    r_sdr: &[f32],
    g_sdr: &[f32],
    b_sdr: &[f32],
) {
    let hdr_vec = pack_rgba32f_bytes(r_hdr, g_hdr, b_hdr);
    let sdr_vec = pack_rgba32f_bytes(r_sdr, g_sdr, b_sdr);

    let widthu: u32 = width as u32;
    let heightu = height as u32;

    let hdr = UhdrRawImage::from_data(
        widthu,
        heightu,
        UhdrPixelFormat::Rgba32F,
        UhdrColorGamut::Bt2020,
        UhdrColorTransfer::Linear,
        hdr_vec,
    )
    .unwrap();

    let sdr = UhdrRawImage::from_data(
        widthu,
        heightu,
        UhdrPixelFormat::Rgba32F,
        UhdrColorGamut::Bt2020,
        UhdrColorTransfer::Linear,
        sdr_vec,
    )
    .unwrap();

    let stop = MyStop { stop: false };

    let (gainmap, metadata) =
        compute_gainmap(&hdr, &sdr, &GainMapConfig::default(), &stop).unwrap();
    //stop.check();

    let sdr_8 = linear_bt2020_to_rgb8(widthu, heightu, &r_sdr, &g_sdr, &b_sdr);

    // Layer 1: Reusable config
    let config = EncoderConfig::ycbcr(85, ChromaSubsampling::None).progressive(true);

    // Step 3-6: Encode and assemble
    let sdr_jpeg = encode_with_gainmap(&sdr_8, &gainmap, &metadata, &config, 75.0, &stop).unwrap();

    std::fs::write(&to_file, &sdr_jpeg).unwrap();
}
*/