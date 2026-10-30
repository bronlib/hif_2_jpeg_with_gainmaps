use std::fs;

use crate::create_png::create_pngu8;
use enough::{Stop, StopReason, Unstoppable};
use std::time::Instant;
use ultrahdr_core::{GainMapEncodingFormat, RawImage};
use zenjpeg::encoder::{ChromaSubsampling, EncoderConfig};
use zenjpeg::ultrahdr::{
    GainMap, GainMapConfig, UhdrColorGamut, UhdrColorTransfer, UhdrPixelFormat, UhdrRawImage,
    compute_gainmap, encode_with_gainmap, encode_with_gainmap_format, generate_gainmap_xmp,
};

use crate::color_transform::bt2020_to_bt709;
use crate::create_jpeg_from_hdr::create_hdr_from_file;
use crate::gain_mapping::compute_mygainmap;
use crate::gainmap_jpeg_fix::{add_iso21496_to_xmp_ultrahdr, linear_bt2020_icc_profile};
use crate::transfer_functions::apply_transfer;
use oxideav_pixfmt::transfer::{bt709_inverse_oetf, bt1886_inverse_eotf};
use zenjpeg::decoder::{DecodedImage, Decoder};

fn get_linear_rgb(img: &UhdrRawImage, x: u32, y: u32) -> [f32; 3] {
    match img.format {
        UhdrPixelFormat::Rgba8 | UhdrPixelFormat::Rgb8 => {
            let bpp = if img.format == UhdrPixelFormat::Rgba8 {
                4
            } else {
                3
            };
            let idx = (y * img.stride + x * bpp as u32) as usize;
            let r = img.data[idx] as f32 / 255.0;
            let g = img.data[idx + 1] as f32 / 255.0;
            let b = img.data[idx + 2] as f32 / 255.0;

            // Apply EOTF based on transfer function
            match img.transfer {
                UhdrColorTransfer::Linear => [r, g, b],
                _ => [r, g, b], // Assume sRGB for 8-bit
            }
        }

        UhdrPixelFormat::Rgba32F => {
            let idx = (y * img.stride + x * 16) as usize;
            let r = f32::from_le_bytes([
                img.data[idx],
                img.data[idx + 1],
                img.data[idx + 2],
                img.data[idx + 3],
            ]);
            let g = f32::from_le_bytes([
                img.data[idx + 4],
                img.data[idx + 5],
                img.data[idx + 6],
                img.data[idx + 7],
            ]);
            let b = f32::from_le_bytes([
                img.data[idx + 8],
                img.data[idx + 9],
                img.data[idx + 10],
                img.data[idx + 11],
            ]);
            [r, g, b]
        }
        _ => {
            println!("unknown format");
            [0.0, 0.0, 0.0]
        }
    }
}

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

fn get_pixels_rgb32f(file: &str) -> UhdrRawImage {
    let jpeg_data = fs::read(file).unwrap();

    let decoded = Decoder::new().decode(&jpeg_data, Unstoppable).unwrap();

    let width = decoded.width;
    let height = decoded.height;

    let rgb: Vec<u8> = decoded.pixels_u8().expect("Expected 8-bit image").to_vec();

    let mut rgba_bytes = Vec::with_capacity(width as usize * height as usize * 16);

    for p in rgb.chunks_exact(3) {
        let r = p[0] as f32 / 255.0;
        let g = p[1] as f32 / 255.0;
        let b = p[2] as f32 / 255.0;
        let a = 1.0f32;

        rgba_bytes.extend_from_slice(&r.to_ne_bytes());
        rgba_bytes.extend_from_slice(&g.to_ne_bytes());
        rgba_bytes.extend_from_slice(&b.to_ne_bytes());
        rgba_bytes.extend_from_slice(&a.to_ne_bytes());
    }

    UhdrRawImage::from_data(
        width,
        height,
        UhdrPixelFormat::Rgba32F,
        UhdrColorGamut::Bt2020,
        UhdrColorTransfer::Linear,
        rgba_bytes,
    )
    .unwrap()
}
fn get_pixels_rgb8(file: &str) -> UhdrRawImage {
    let jpeg_data = fs::read(file).unwrap();

    let decoded = Decoder::new().decode(&jpeg_data, Unstoppable).unwrap();

    let width = decoded.width;
    let height = decoded.height;

    let rgb: Vec<u8> = decoded.pixels_u8().expect("Expected 8-bit image").to_vec();

    UhdrRawImage::from_data(
        width,
        height,
        UhdrPixelFormat::Rgb8,
        UhdrColorGamut::Bt2020,
        UhdrColorTransfer::Linear,
        rgb,
    )
    .unwrap()
}

pub(crate) fn create_from_2_files(
    hdr_file: &str,
    sdr_file: &str,
    to_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let total_start = Instant::now();
    println!(
        "create_from_2_file {}, {} to file {}",
        hdr_file, sdr_file, to_file
    );

    let hdr = get_pixels_rgb32f(hdr_file);
    let sdr = get_pixels_rgb32f(sdr_file);

    create_from_2_raw(hdr, sdr, to_file)?;
    Ok(())
}

pub(crate) fn create_from_2_raw(
    hdr: RawImage,
    sdr: RawImage,
    to_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let total_start = Instant::now();
    println!("create_from_2_raw : to file {}", to_file);

    let stop = MyStop { stop: false };

    let mut gainmap_config = GainMapConfig::default();
    gainmap_config.scale_factor = 1;
    gainmap_config.multi_channel = true;
    gainmap_config.gamma = 1.0;
    gainmap_config.min_boost = 0.0;
    gainmap_config.max_boost = 2.0;
    gainmap_config.base_offset = 1.0 / 64.0;
    gainmap_config.alternate_offset = 1.0 / 64.0;
    gainmap_config.base_hdr_headroom = 0.0;
    gainmap_config.alternate_hdr_headroom = 3.0;

    let (gainmap, metadata) = compute_mygainmap(&hdr, &sdr, &gainmap_config, &stop).unwrap();
    //stop.check();

    let sdr_709 = bt2020_to_bt709(&sdr);
    let sdr_e = apply_transfer(&sdr_709, &bt1886_inverse_eotf);
    let sdr_8 = rgba32f_to_rgb8(&sdr_e);

    // Layer 1: Reusable config
    let config = EncoderConfig::ycbcr(85, ChromaSubsampling::None).progressive(true);

    println!(" min log {:?}", metadata.gain_map_min);
    println!(" max log {:?}", metadata.gain_map_max);
    println!(" gamma {:?}", metadata.gamma);
    println!(" sdr offset {:?}", metadata.base_offset);
    println!(" base hdr headroom {:?}", metadata.base_hdr_headroom);
    println!(" min boost: {:?}", gainmap_config.min_boost);
    println!(" max boost: {:?}", gainmap_config.max_boost);

    // Step 3-6: Encode and assemble
    let sdr_jpeg = encode_with_gainmap_format(
        &sdr_8,
        &gainmap,
        &metadata,
        &config,
        95.0,
        GainMapEncodingFormat::Xmp,
        &stop,
    )
    .unwrap();

    // Keep XMP as the encoder format for browser compatibility, then add the
    // standards-based ISO 21496-1 APP2 blocks using the exact same metadata.
    let alternate_space_icc = linear_bt2020_icc_profile();
    let sdr_jpeg = add_iso21496_to_xmp_ultrahdr(&sdr_jpeg, &metadata, &alternate_space_icc)?;

    std::fs::write(&to_file, &sdr_jpeg).unwrap();

    let rgb = gainmap.data;
    let width = gainmap.width;
    let height = gainmap.height;
    let len = rgb.len();
    let pixels = rgb.len() / 3;
    let mut r = Vec::with_capacity(pixels);
    let mut g = Vec::with_capacity(pixels);
    let mut b = Vec::with_capacity(pixels);

    for chunk in rgb.chunks_exact(3) {
        r.push(chunk[0]);
        g.push(chunk[1]);
        b.push(chunk[2]);
    }
    create_pngu8(
        "gainmapmy_h2.png",
        width as usize,
        height as usize,
        &r,
        &g,
        &b,
    )
    .unwrap();

    let total = total_start.elapsed();
    println!("Total pipeline time: {:.3?}", total);
    Ok(())
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

pub(crate) fn encodemy_zen(
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
    println!("Encoding Zen to file : {}", to_file);
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

    create_from_2_raw(hdr, sdr, &to_file).unwrap();
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

pub(crate) fn rgba32f_to_rgb8(image: &RawImage) -> RawImage {
    assert_eq!(image.format, UhdrPixelFormat::Rgba32F);

    let mut data = Vec::with_capacity(image.width as usize * image.height as usize * 3);

    for pixel in image.data.chunks_exact(16) {
        let r = f32::from_ne_bytes(pixel[0..4].try_into().unwrap());
        let g = f32::from_ne_bytes(pixel[4..8].try_into().unwrap());
        let b = f32::from_ne_bytes(pixel[8..12].try_into().unwrap());

        data.push((r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
        data.push((g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
        data.push((b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
    }

    RawImage::from_data(
        image.width,
        image.height,
        UhdrPixelFormat::Rgb8,
        image.gamut,
        image.transfer,
        data,
    )
    .unwrap()
}
