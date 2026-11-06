
use enough::{Stop, StopReason};
use ultrahdr_core::{ColorTransfer, GainMapEncodingFormat, RawImage};
use zenjpeg::encoder::{ChromaSubsampling, EncoderConfig};
use zenjpeg::ultrahdr::{
    GainMapConfig, UhdrColorGamut, UhdrColorTransfer, UhdrPixelFormat, UhdrRawImage,
    encode_with_gainmap_format
};

use crate::color_transform::{bt2020_to_bt709, bt709_to_bt2020 };
use crate::gain_mapping::compute_mygainmap;
use crate::gainmap_jpeg_fix::{add_iso21496_to_xmp_ultrahdr, linear_bt2020_icc_profile};
use crate::transfer_functions::apply_transfer;
use oxideav_pixfmt::transfer::{ bt1886_inverse_eotf};
use std::path::PathBuf;
use image::{DynamicImage, ImageResult};

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

/*
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

*/

pub(crate) fn create_from_2_raw(
    hdr: RawImage,
    sdr: RawImage,
    to_file: &PathBuf,
    create_gainmapped_jpg: bool,
) -> ImageResult<DynamicImage> {
    println!("create_from_2_raw : to file {:?}", to_file);
    assert_eq!(sdr.gamut, UhdrColorGamut::Bt2020);
    /*
    Reduce the gammut so that the gainmap will include the
    information needed to restore the full 2020 gamut of the
    original picture
     */
    let sdr709=bt2020_to_bt709(&sdr);
    let sdr2020 = bt709_to_bt2020(&sdr709);

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

    let sdr_709 = bt2020_to_bt709(&sdr2020);
    let mut sdr_e = apply_transfer(&sdr_709, &bt1886_inverse_eotf);
    sdr_e.transfer = ColorTransfer::Srgb;
    let sdr_8 = rgba32f_to_rgb8(&sdr_e);

    // Layer 1: Reusable config
    let config = EncoderConfig::ycbcr(85, ChromaSubsampling::None).progressive(true);

    if create_gainmapped_jpg {
        let (gainmap, metadata) = compute_mygainmap(&hdr, &sdr2020, &gainmap_config, &stop)
            .map_err(|error| image::ImageError::IoError(std::io::Error::other(error.to_string())))?;

        let rgb = &gainmap.data;
        let pixels = rgb.len() / 3;
        let mut r = Vec::with_capacity(pixels);
        let mut g = Vec::with_capacity(pixels);
        let mut b = Vec::with_capacity(pixels);
        for chunk in rgb.chunks_exact(3) {
            r.push(chunk[0]);
            g.push(chunk[1]);
            b.push(chunk[2]);
        }
       
        let sdr_jpeg = encode_with_gainmap_format(
            &sdr_8,
            &gainmap,
            &metadata,
            &config,
            95.0,
            GainMapEncodingFormat::Xmp,
            &stop,
        )
        .map_err(|error| image::ImageError::IoError(std::io::Error::other(error.to_string())))?;

        let alternate_space_icc = linear_bt2020_icc_profile();
        let sdr_jpeg = add_iso21496_to_xmp_ultrahdr(&sdr_jpeg, &metadata, &alternate_space_icc)
            .map_err(|error| image::ImageError::IoError(std::io::Error::other(error.to_string())))?;
        std::fs::write(to_file, &sdr_jpeg)?;
    }

    let raw: RawImage = sdr_8;

    let rgb = image::RgbImage::from_raw(raw.width, raw.height, raw.data)
        .ok_or_else(|| {
            image::ImageError::Parameter(image::error::ParameterError::from_kind(
                image::error::ParameterErrorKind::DimensionMismatch,
            ))
        }).unwrap();

    Ok(image::DynamicImage::ImageRgb8(rgb))
}

/// Pack f32 RGB → RGBA32F → Vec<u8> (for ultrahdr-rs 0.3.5)
pub fn pack_rgba32f_bytes(r: &[f32], g: &[f32], b: &[f32]) -> Vec<u8> {
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

pub(crate) fn encode_my_zen(
    to_file: &PathBuf,
    width: usize,
    height: usize,
    r_hdr: &[f32],
    g_hdr: &[f32],
    b_hdr: &[f32],
    r_sdr: &[f32],
    g_sdr: &[f32],
    b_sdr: &[f32],
    create_gainmapped_jpg: bool,
) -> ImageResult<DynamicImage> {
    println!("Encoding Zen to file : {:?}", to_file);
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

    create_from_2_raw(hdr, sdr, to_file, create_gainmapped_jpg)
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
