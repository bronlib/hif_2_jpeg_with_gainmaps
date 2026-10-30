use zenjpeg::encoder::{ChromaSubsampling, EncoderConfig};
use zenjpeg::ultrahdr::{
    GainMapConfig, ToneMapConfig, UhdrColorGamut, UhdrColorTransfer, UhdrPixelFormat, UhdrRawImage,
    compute_gainmap, encode_ultrahdr, encode_with_gainmap,
};

use core::sync::atomic::{AtomicBool, Ordering};
use enough::{Stop, StopReason};

struct MyStop {
    stop: bool,
}

impl Stop for MyStop {
    fn check(&self) -> Result<(), StopReason> {
        if self.stop {
            Err(StopReason::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// Pack f32 RGB → RGBA32F → Vec<u8> (for ultrahdr-rs 0.3.5)
fn pack_rgba32f_bytes(r: &[f32], g: &[f32], b: &[f32]) -> Vec<u8> {
    let n = r.len();
    let mut out = Vec::with_capacity(n * 16);

    for i in 0..n {
        out.extend_from_slice(&r[i].to_le_bytes());
        out.extend_from_slice(&g[i].to_le_bytes());
        out.extend_from_slice(&b[i].to_le_bytes());
        out.extend_from_slice(&1.0f32.to_le_bytes()); // alpha
    }

    out
}

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

fn linear_bt2020_to_rgb8(
    width: u32,
    height: u32,
    r_sdr: &[f32],
    g_sdr: &[f32],
    b_sdr: &[f32],
) -> UhdrRawImage {
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
