//! Decode regular JPEGs and Ultra HDR JPEGs for display in the Slint viewer.
//!
//! Full-image decoding supports progressive JPEGs. This module returns the
//! base image and gain-map data, plus reconstructed linear BT.2020 HDR pixels.

use std::{ fs, path::PathBuf};

use enough::{Stop, Unstoppable};
use image::{DynamicImage};
use zenjpeg::{
    decoder::Decoder,
    ultrahdr::{GainMap, GainMapMetadata, UltraHdrExtras},
};


use color_transform::bt2020_to_bt709;
use crate::create_jpeg_from_2_files::rgba32f_to_rgb8;


use zenjpeg::ultrahdr::{UhdrPixelFormat, UhdrRawImage};

use crate::{color_transform};
use crate::raw_image_transforms::{raw_image_to_rgb, rgb_to_raw_image};

use oxideav_pixfmt::transfer::{bt1886_eotf, bt1886_inverse_eotf};
use ultrahdr_core::{ColorTransfer, PixelFormat, RawImage};

use zenjpeg::ultrahdr::{ UhdrColorGamut, UhdrColorTransfer};
use crate::transfer_functions::apply_transfer;

#[derive(Default)]
#[derive(Clone)]
pub struct

DecodedJpeg {
    pub width: u32,
    pub height: u32,
    /// Decoded base JPEG pixels in the decoder's native RGB output.
    pub rgb: Vec<u8>,
    pub has_gain_map: bool,
    /// Decoded gain map, retaining its native dimensions and channel count.
    pub gain_map: Option<GainMap>,
    /// Parsed Ultra HDR gain-map metadata (ISO/XMP fields).
    pub metadata: Option<GainMapMetadata>,
    /// Original gain-map JPEG bytes extracted from the MPF container.
    pub gainmap_jpeg: Option<Vec<u8>>,
    /// XMP packet preserved from the primary image, when present.
    pub xmp: Option<String>,
    /// ICC profile from the primary image, when present; useful for gamut handling.
    pub icc_profile: Option<Vec<u8>>,
    /// Per-channel log2 gains at native gain-map resolution.
    pub log2_gain_map: Option<Vec<[f32; 3]>>,
    /// Per-channel gain multipliers at the selected display boost.
    pub gain_multipliers: Option<Vec<[f32; 3]>>,
    /// Reconstructed linear BT.2020 HDR image. The RGB8 base remains untouched.
    pub hdr_linear_bt2020: Option<zenjpeg::ultrahdr::UhdrRawImage>,
}

impl DecodedJpeg {

    pub fn create_dyn(self)-> Result<DynamicImage, String>{
        let rgb = image::RgbImage::from_raw(self.width, self.height, self.rgb)
            .ok_or_else(|| {
                image::ImageError::Parameter(image::error::ParameterError::from_kind(
                    image::error::ParameterErrorKind::DimensionMismatch,
                ))
            }).unwrap();
        Ok(image::DynamicImage::ImageRgb8(rgb))
    }
}


pub(crate) fn decode_file(path: &PathBuf) ->Result<DynamicImage, String> {
    let bytes = fs::read(path).unwrap();
    decode_bytes(&bytes)
}

fn decode_bytes(bytes: &[u8]) -> Result<DynamicImage, String>{
    let decoded = Decoder::new()
        .preserve_all_metadata()
        .decode(bytes, Unstoppable)
        .map_err(|error| error.to_string()).unwrap();

    let width = decoded.width();
    let height = decoded.height();
    let sdr = decoded
        .pixels_u8()
        .ok_or_else(|| "JPEG decoder did not return 8-bit pixels".to_owned()).unwrap();
    let rgb = sdr.to_vec();
    let (gain_map, metadata, gainmap_jpeg, xmp, icc_profile) =
        if let Some(extras) = decoded.extras() {
            let gainmap_jpeg = extras.gainmap().map(ToOwned::to_owned);
            let has_ultrahdr = extras.is_ultrahdr() || gainmap_jpeg.is_some();
            let metadata = if has_ultrahdr {
                extras
                    .ultrahdr_metadata()
                    .map(|result| result.map(|(metadata, _)| metadata))
                    .transpose()
                    .map_err(|error| error.to_string()).unwrap()
            } else {
                None
            };
            let gain_map = if has_ultrahdr {
                extras
                    .decode_gainmap()
                    .map(|result| result.map_err(|error| error.to_string()))
                    .transpose().unwrap()
            } else {
                None
            };
            let xmp = extras.xmp().map(ToOwned::to_owned);
            let icc_profile = extras.icc_profile().map(ToOwned::to_owned);
            (gain_map, metadata, gainmap_jpeg, xmp, icc_profile)
        } else {
            (None, None, None, None, None)
        };
    let has_gain_map = gain_map.is_some() || gainmap_jpeg.is_some();

    let inp_decoded_jpeg = DecodedJpeg {
        width,
        height,
        rgb,
        has_gain_map,
        gain_map,
        metadata,
        gainmap_jpeg,
        xmp,
        icc_profile,
        log2_gain_map: None,
        gain_multipliers: None,
        hdr_linear_bt2020: None,
    };

    process_decoded_jpeg(inp_decoded_jpeg)

}

fn process_decoded_jpeg(inp: DecodedJpeg) -> Result<DynamicImage, String> {


    let rgbres = if inp.metadata.is_some()
    {
        transfer_gain(inp)
    } else {
        let rgb = image::RgbImage::from_raw(inp.width, inp.height, inp.rgb)
            .ok_or_else(|| {
                image::ImageError::Parameter(image::error::ParameterError::from_kind(
                    image::error::ParameterErrorKind::DimensionMismatch,
                ))
            }).unwrap();

        Ok(image::DynamicImage::ImageRgb8(rgb))
    };
    rgbres
}
pub fn transfer_gain(inp:DecodedJpeg)-> Result<DynamicImage, String>
{
    println!("Now in transfer_gain");
    apply_multi_channel_gain(inp)
}



/// Decode the gain-map samples and apply them to reconstruct linear BT.2020 HDR.
///
/// The JPEG base produced by this project is BT.709 with a BT.1886 transfer.
/// The gain map is applied in its declared color space, then the result is
/// returned in BT.2020 linear light. The RGB8 base image remains unchanged.
pub(crate) fn apply_multi_channel_gain(
    inp: DecodedJpeg,
) -> Result<DynamicImage, String>{
    let (gain_map, metadata) = match (&inp.gain_map, &inp.metadata) {
        (Some(gain_map), Some(metadata)) => (gain_map, metadata),
        (None, None) if !inp.has_gain_map => return inp.create_dyn(),
        _ => return Err("Gain-map pixels or metadata are missing".into()),
    };

    let mut final_jpeg = inp.clone();


    let weight = 1.0;
    let (_, gain_multipliers) = decode_gain_samples(gain_map, metadata, weight)?;


    let expected_gain_bytes = (gain_map.width as usize)
        .checked_mul(gain_map.height as usize)
        .and_then(|pixels| pixels.checked_mul(gain_map.channels as usize))
        .ok_or_else(|| "Gain-map dimensions overflow".to_owned())?;
    if gain_map.channels != 1 && gain_map.channels != 3 {
        return Err(format!(
            "Unsupported gain-map channel count: {}",
            gain_map.channels
        ));
    }
    if gain_map.data.len() != expected_gain_bytes {
        return Err(format!(
            "Gain-map data length mismatch: expected {expected_gain_bytes}, got {}",
            gain_map.data.len()
        ));
    }

    let sdr_base_linear = decoded_base_to_linear(&inp)?;
    let base_for_apply = linear_to_bt2020(&sdr_base_linear)?;


    let hdr_in_gainmap_space = my_apply_gainmap(
                                                &base_for_apply,
                                                metadata,
                                                gain_multipliers,
                                                Unstoppable,
                                            )
        .map_err(|error| error.to_string())?;
    println!("hdr_in_gainmap_space  len = {}", hdr_in_gainmap_space.data.len());
    let hdr_linear_b709 = bt2020_to_bt709(&hdr_in_gainmap_space);
    let sdr_e = apply_transfer(&hdr_linear_b709, &bt1886_inverse_eotf);
    let hdr_u8 =  rgba32f_to_rgb8(&sdr_e);

    final_jpeg.rgb = hdr_u8.data;


    println!("inp.rgb len = {}, hdr_8 len={}", final_jpeg.rgb.len(), inp.rgb.len());

    //assert_eq!(final_jpeg.rgb.len() as u32, width * height * 3);

    println!("now at the end of apply_multi_channel_gain ");

    final_jpeg.create_dyn()

}

fn decode_gain_samples(
    gain_map: &GainMap,
    metadata: &GainMapMetadata,
    weight: f32,
) -> Result<(Vec<[f32; 3]>, Vec<[f32; 3]>), String> {
    let pixel_count = gain_map.width as usize * gain_map.height as usize;
    let mut log2_values = Vec::with_capacity(pixel_count);
    let mut multipliers = Vec::with_capacity(pixel_count);

    for pixel in 0..pixel_count {
        let mut log2_gain = [0.0; 3];
        let mut multiplier = [0.0; 3];
        for channel in 0..3 {
            let map_channel = if gain_map.channels == 1 { 0 } else { channel };
            let metadata_channel = if gain_map.channels == 1 { 0 } else { channel };
            let gamma = metadata.gamma[metadata_channel] as f32;
            if !gamma.is_finite() || gamma <= 0.0 {
                return Err(format!("Invalid gain-map gamma for channel {channel}"));
            }
            let index = pixel * gain_map.channels as usize + map_channel;
            let normalized = gain_map.data[index] as f32 / 255.0;
            let norm = normalized.powf(1.0 / gamma);
            let log_gain = metadata.gain_map_min[metadata_channel] as f32
                + norm
                * (metadata.gain_map_max[metadata_channel]
                - metadata.gain_map_min[metadata_channel]) as f32;
            log2_gain[channel] = log_gain;
            multiplier[channel] = (log_gain * weight).exp2();
        }
        log2_values.push(log2_gain);
        multipliers.push(multiplier);
    }

    Ok((log2_values, multipliers))
}

fn decoded_base_to_linear(inp: &DecodedJpeg) -> Result<UhdrRawImage, String> {
    let pixel_count = (inp.width as usize)
        .checked_mul(inp.height as usize)
        .ok_or_else(|| "Base-image dimensions overflow".to_owned())?;
    if inp.rgb.len() != pixel_count * 3 {
        return Err("Decoded JPEG pixel data is not packed RGB8".into());
    }

    let mut r = Vec::with_capacity(pixel_count);
    let mut g = Vec::with_capacity(pixel_count);
    let mut b = Vec::with_capacity(pixel_count);
    if inp.icc_profile.is_some() {
        return Err(
            "The primary JPEG has an ICC profile; convert its base image to a known gamut first"
                .into(),
        );
    }
    for pixel in inp.rgb.chunks_exact(3) {
        r.push(bt1886_eotf(pixel[0] as f32 / 255.0));
        g.push(bt1886_eotf(pixel[1] as f32 / 255.0));
        b.push(bt1886_eotf(pixel[2] as f32 / 255.0));
    }

    rgb_to_raw_image(
        inp.width as usize,
        inp.height as usize,
        &r,
        &g,
        &b,
        UhdrPixelFormat::Rgba32F,
        UhdrColorGamut::Bt709,
        UhdrColorTransfer::Linear,
    )
}

fn linear_to_bt2020(image: &UhdrRawImage) -> Result<UhdrRawImage, String> {
    if image.gamut == UhdrColorGamut::Bt2020 {
        return Ok(image.clone());
    }
    let (r709, g709, b709) = raw_image_to_rgb(image)?;
    let mut r2020 = Vec::with_capacity(r709.len());
    let mut g2020 = Vec::with_capacity(r709.len());
    let mut b2020 = Vec::with_capacity(r709.len());
    for ((r, g), b) in r709.iter().zip(&g709).zip(&b709) {
        let converted = match image.gamut {
            UhdrColorGamut::Bt709 => [
                0.627_403_9 * r + 0.329_283_0 * g + 0.043_313_1 * b,
                0.069_097_3 * r + 0.919_540_4 * g + 0.011_362_3 * b,
                0.016_391_4 * r + 0.088_013_3 * g + 0.895_595_3 * b,
            ],
            UhdrColorGamut::DisplayP3 => [
                0.753_833_0 * r + 0.198_597_4 * g + 0.047_569_6 * b,
                0.045_743_8 * r + 0.941_777_2 * g + 0.012_479_0 * b,
                -0.001_210_3 * r + 0.017_601_7 * g + 0.983_608_6 * b,
            ],
            UhdrColorGamut::Bt2020 => unreachable!(),
        };
        r2020.push(converted[0]);
        g2020.push(converted[1]);
        b2020.push(converted[2]);
    }
    rgb_to_raw_image(
        image.width as usize,
        image.height as usize,
        &r2020,
        &g2020,
        &b2020,
        UhdrPixelFormat::Rgba32F,
        UhdrColorGamut::Bt2020,
        UhdrColorTransfer::Linear,
    )
}


/// Apply gain to SDR pixel to get HDR.
///
///log2_gain_map[f32; 3]:, gain_multipliers
///
fn apply_gain(sdr_linear: [f32; 3], multipliers: [f32; 3], metadata: &GainMapMetadata) -> [f32; 3] {
    [
        (sdr_linear[0] + metadata.base_offset[0] as f32) * multipliers[0]
            - metadata.alternate_offset[0] as f32,
        (sdr_linear[1] + metadata.base_offset[1] as f32) * multipliers[1]
            - metadata.alternate_offset[1] as f32,
        (sdr_linear[2] + metadata.base_offset[2] as f32) * multipliers[2]
            - metadata.alternate_offset[2] as f32,
    ]
}



/// Apply a gain map to an SDR image to reconstruct HDR.
///
/// The `display_boost` parameter controls how much HDR effect to apply:
/// - 1.0 = SDR output (no boost)
/// - 2.0 = 2x brightness capability
/// - 4.0 = 4x brightness capability (typical HDR display)
///
/// The `stop` parameter enables cooperative cancellation. Pass `Unstoppable`
/// when cancellation is not needed.
pub fn my_apply_gainmap(
    sdr: &RawImage,
    metadata: &GainMapMetadata,
    gain_multipliers: Vec<[f32; 3]>,
    stop: impl Stop,
) -> ultrahdr_core::Result<RawImage> {
    // Validate pixel data is large enough for declared dimensions
    sdr.validate_data_bounds()?;

    let width = sdr.width;
    let height = sdr.height;
    let mut img = RawImage::new(width, height, PixelFormat::Rgba32F)?;
    img.transfer = ColorTransfer::Linear;
    img.gamut = sdr.gamut;
    let mut output = img;


    // Process each row, checking for cancellation periodically
    for y in 0..height {
        // Check for cancellation once per row (not per pixel for performance)
        stop.check()?;

        for x in 0..width {
            // Get SDR pixel (convert to linear)
            // Get SDR pixel (convert to linear)
            let sdr_linear = get_sdr_linear(sdr, x, y);

            let pixel = (y*width + x)  as usize;
            let multipliers =gain_multipliers[pixel];
            // Apply gain to reconstruct HDR
            let hdr_linear = apply_gain(sdr_linear, multipliers, metadata);

            // Write output
            write_linear_float(&mut output, x, y, hdr_linear);
        }
    }

    Ok(output)
}

/// Write linear f32 RGBA to output.
#[inline]
fn write_linear_float(output: &mut RawImage, x: u32, y: u32, hdr: [f32; 3]) {
    let idx = (y * output.stride + x * 16) as usize;
    let r_bytes = hdr[0].to_le_bytes();
    let g_bytes = hdr[1].to_le_bytes();
    let b_bytes = hdr[2].to_le_bytes();
    let a_bytes = 1.0f32.to_le_bytes();

    output.data[idx..idx + 4].copy_from_slice(&r_bytes);
    output.data[idx + 4..idx + 8].copy_from_slice(&g_bytes);
    output.data[idx + 8..idx + 12].copy_from_slice(&b_bytes);
    output.data[idx + 12..idx + 16].copy_from_slice(&a_bytes);
}


/// Get linear RGB from SDR image.
fn get_sdr_linear(img: &RawImage, x: u32, y: u32) -> [f32; 3] {

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

