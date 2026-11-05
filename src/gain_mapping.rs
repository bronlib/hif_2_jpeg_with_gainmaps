//! Gain map computation from HDR and SDR images.


use enough::{Stop};

use zenjpeg::ultrahdr::{
    GainMap, GainMapMetadata, UhdrPixelFormat, UhdrRawImage,
};

use zenjpeg::ultrahdr::GainMapConfig;
use zenjpeg::ultrahdr::UhdrColorGamut::Bt2020;
use get_headroom::headroom_2bt2020raw;
use crate::get_headroom;

/// Compute a gain map from HDR and SDR images.
///
/// The gain map represents the ratio between HDR and SDR pixel values,
/// encoded as 8-bit values in the range `[0, 255]`.
///
/// The `stop` parameter enables cooperative cancellation. Pass `Unstoppable`
/// when cancellation is not needed.
///

/// sRGB OETF: Linear `[0,1]` → sRGB encoded `[0,1]`
///
/// Delegates to `linear-srgb` (rational polynomial, ~5e-7 max error, no `powf`).

/// sRGB EOTF (inverse OETF): sRGB encoded `[0,1]` → Linear `[0,1]`
///
/// Delegates to `linear-srgb` (rational polynomial, ~5e-7 max error, no `powf`).
pub fn compute_mygainmap(
    hdr: &UhdrRawImage,
    sdr: &UhdrRawImage,
    config: &GainMapConfig,
    stop: impl Stop,
    progress: &dyn Fn(&str),
) -> Result<(GainMap, GainMapMetadata), Box<dyn std::error::Error>> {
    // Validate pixel data is large enough for declared dimensions
    hdr.validate_data_bounds()?;
    sdr.validate_data_bounds()?;

    let scale = config.scale_factor.max(1) as u32;
    let gm_width = hdr.width.div_ceil(scale);
    let gm_height = hdr.height.div_ceil(scale);

    println!("scale: {} ", scale);
    println!("gm_width: {} ", gm_width);
    println!("gm_height: {} ", gm_height);
    println!("hdr.width: {} ", hdr.width);
    println!("hdr.height: {} ", hdr.height);

    // Track actual min/max boost values found
    let mut actual_min_boost = f32::MAX;
    let mut actual_max_boost = f32::MIN;

    // Compute gain map
    let (gainmap, max_log, min_log) = mycompute_multichannel_gainmap(
        hdr,
        sdr,
        gm_width,
        gm_height,
        scale,
        config,
        &mut actual_min_boost,
        &mut actual_max_boost,
        &stop,
        progress,
    )?;

    // Clamp actual values to configured range

    //actual_min_boost = 0.0;
    //actual_max_boost = 2.0;

    // Build metadata (convert linear boost values to log2 domain)
    let mut metadata = GainMapMetadata::default();
    metadata.gain_map_min = min_log;
    metadata.gain_map_max = max_log;
    metadata.gamma = [config.gamma as f64; 3];
    metadata.base_offset = [config.base_offset as f64; 3];
    metadata.alternate_offset = [config.alternate_offset as f64; 3];
    // Headroom fields in GainMapMetadata are log2(display boost), while the
    // public GainMapConfig values are linear boost ratios.
    metadata.base_hdr_headroom = (config.base_hdr_headroom.max(1.0) as f64).log2();
    metadata.alternate_hdr_headroom = headroom_2bt2020raw(hdr, sdr);
    metadata.use_base_color_space = false;
    metadata.backward_direction = false;

    Ok((gainmap, metadata))
}

/// Compute multi-channel (RGB) gain map.

fn mycompute_multichannel_gainmap(
    hdr: &UhdrRawImage,
    sdr: &UhdrRawImage,
    gm_width: u32,
    gm_height: u32,
    scale: u32,
    config: &GainMapConfig,
    actual_min_boost: &mut f32,
    actual_max_boost: &mut f32,
    stop: &impl Stop,
    progress: &dyn Fn(&str),
) -> Result<(GainMap, [f64; 3], [f64; 3]), Box<dyn std::error::Error>> {
    let mut gainmap = GainMap::new_multichannel(gm_width, gm_height)?;


    *actual_min_boost = 0.0;
    *actual_max_boost = 2.0;

    let offset_sdr: [f32; 3] = [config.base_offset, config.base_offset, config.base_offset];
    let offset_hdr: [f32; 3] = [config.base_offset, config.base_offset, config.base_offset];
    let gamma: [f32; 3] = [config.gamma, config.gamma, config.gamma];

    // Temporary storage to hold raw log-gains for every pixel so we can find min/max
    let mut log_gains = vec![vec![[0.0f32; 3]; gm_width as usize]; gm_height as usize];

    let mut min_log = [f32::INFINITY; 3];
    let mut max_log = [f32::NEG_INFINITY; 3];

    if hdr.gamut != Bt2020 {
        println!("Warning, hdr gamut should be BT.2020");
    }
    if sdr.gamut != Bt2020 {
        println!("Warning, sdr gamut should be BT.2020");
    };

    if scale != 1 {
        println!("ERROR scale: {}", scale);
    }

    for gy in 0..gm_height {
        // Check for cancellation once per row
        stop.check().unwrap();

        for gx in 0..gm_width {
            let x = gx; // (gx * scale + scale / 2).min(hdr.width - 1);
            let y = gy; // (gy * scale + scale / 2).min(hdr.height - 1);

            let hdr_rgb = get_linear_rgb(hdr, x, y);
            let sdr_rgb = get_linear_rgb(sdr, x, y);

            // Calculate gain factor and convert to log2 space
            let factor_r = (hdr_rgb[0] + offset_hdr[0]) / (sdr_rgb[0] + offset_sdr[0]);
            let factor_g = (hdr_rgb[1] + offset_hdr[1]) / (sdr_rgb[1] + offset_sdr[1]);
            let factor_b = (hdr_rgb[2] + offset_hdr[2]) / (sdr_rgb[2] + offset_sdr[2]);

            let log_r = factor_r.max(1e-6).log2();
            let log_g = factor_g.max(1e-6).log2();
            let log_b = factor_b.max(1e-6).log2();

            log_gains[gy as usize][gx as usize] = [log_r, log_g, log_b];

            // Track min and max
            min_log[0] = min_log[0].min(log_r);
            max_log[0] = max_log[0].max(log_r);
            min_log[1] = min_log[1].min(log_g);
            max_log[1] = max_log[1].max(log_g);
            min_log[2] = min_log[2].min(log_b);
            max_log[2] = max_log[2].max(log_b);
        }
        let interval = (gm_height / 100).max(1);
        if gy % interval == 0 || gy + 1 == gm_height {
            let percent = (gy + 1) * 100 / gm_height.max(1);
            progress(&format!("Analyzing gain map: {percent}%"));
        }
    }

    // Pass 2: Normalize log-gains using the dynamically calculated min/max bounds
    progress("Packing gain-map pixels…");
    for gy in 0..gm_height {
        for gx in 0..gm_width {
            let log_val = log_gains[gy as usize][gx as usize];

            let norm_r = ((log_val[0] - min_log[0]) / (max_log[0] - min_log[0])).powf(gamma[0]);
            let norm_g = ((log_val[1] - min_log[1]) / (max_log[1] - min_log[1])).powf(gamma[1]);
            let norm_b = ((log_val[2] - min_log[2]) / (max_log[2] - min_log[2])).powf(gamma[2]);

            let gm_r = (norm_r.clamp(0.0, 1.0) * 255.0).round() as u8;
            let gm_g = (norm_g.clamp(0.0, 1.0) * 255.0).round() as u8;
            let gm_b = (norm_b.clamp(0.0, 1.0) * 255.0).round() as u8;

            let rgb: [u8; 3] = [gm_r, gm_g, gm_b];
            for c in 0..3 {
                let encoded = rgb[c];
                let idx = (gy * gm_width + gx) as usize * 3 + c;
                gainmap.data[idx] = encoded;
            }
        }
        let interval = (gm_height / 100).max(1);
        if gy % interval == 0 || gy + 1 == gm_height {
            let percent = (gy + 1) * 100 / gm_height.max(1);
            progress(&format!("Packing gain map: {percent}%"));
        }
    }
    println!("max_log: {:?}", max_log);
    println!("min_log: {:?}", min_log);
    let maxlog: [f64; 3] = max_log.map(|x| x as f64);
    let minlog: [f64; 3] = min_log.map(|x| x as f64);

    Ok((gainmap, maxlog, minlog))
}

/// Extract linear RGB `[0,1]` from a raw image at the given pixel position.
///
/// When the `transfer` feature is enabled, this function applies appropriate
/// EOTF conversions (sRGB, PQ, HLG) based on the image's transfer function.
///
/// When the `transfer` feature is disabled, only inherently linear formats
/// (Rgba16F, Rgba32F) are supported. For encoded formats, the caller must
/// pre-convert to linear using an external CMS.
/// Extract linear RGB from a raw image (no transfer feature).
///
/// Only supports inherently linear formats (Rgba16F, Rgba32F).
/// For encoded formats (sRGB, PQ, etc.), use an external CMS to pre-convert.

fn get_linear_rgb(img: &UhdrRawImage, x: u32, y: u32) -> [f32; 3] {
    match img.format {
        UhdrPixelFormat::Rgba16F => {
            let idx = (y * img.stride + x * 8) as usize;
            let r = half_to_f32(&img.data[idx..idx + 2]);
            let g = half_to_f32(&img.data[idx + 2..idx + 4]);
            let b = half_to_f32(&img.data[idx + 4..idx + 6]);
            [r, g, b]
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

        // For encoded formats without transfer feature, treat as linear
        // (caller must pre-convert using external CMS)
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
            // Assume already linear - caller must pre-convert
            [r, g, b]
        }

        UhdrPixelFormat::Gray8 => {
            let idx = (y * img.stride + x) as usize;
            let v = img.data[idx] as f32 / 255.0;
            [v, v, v]
        }

        // Formats that require transfer functions - return mid-gray as fallback
        _ => [0.18, 0.18, 0.18],
    }
}

/// Convert half-precision float bytes to f32.
fn half_to_f32(bytes: &[u8]) -> f32 {
    let bits = u16::from_le_bytes([bytes[0], bytes[1]]);
    half::f16::from_bits(bits).to_f32()
}
