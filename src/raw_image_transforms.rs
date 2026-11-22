use ultrahdr_core::RawImage;
use zenjpeg::ultrahdr::{UhdrColorGamut, UhdrColorTransfer, UhdrPixelFormat};

/// Extract the RGB channels from an `Rgba32F` image.
///
/// The `f32` samples are returned as stored. Alpha is ignored, and the image's
/// gamut and transfer metadata remain available on `image` but are not applied.
pub (crate) fn raw_image_to_rgb(
    image: &RawImage,
) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>), String> {
    if image.format != UhdrPixelFormat::Rgba32F {
        return Err(format!(
            "raw_image_to_rgb requires Rgba32F input, got {:?}",
            image.format
        ));
    }

    let width = image.width as usize;
    let height = image.height as usize;
    let count = width
        .checked_mul(height)
        .ok_or_else(|| "RawImage dimensions overflow".to_string())?;
    let stride = image.stride as usize;

    let mut r = Vec::with_capacity(count);
    let mut g = Vec::with_capacity(count);
    let mut b = Vec::with_capacity(count);

    for y in 0..height {
        let row_start = y
            .checked_mul(stride)
            .ok_or_else(|| "RawImage row offset overflow".to_string())?;

        for x in 0..width {
            let pixel_start = row_start
                .checked_add(x * 16)
                .ok_or_else(|| "RawImage pixel offset overflow".to_string())?;
            let pixel = image
                .data
                .get(pixel_start..pixel_start + 16)
                .ok_or_else(|| format!("RawImage data is truncated at ({x}, {y})"))?;

            r.push(f32::from_le_bytes(pixel[0..4].try_into().unwrap()));
            g.push(f32::from_le_bytes(pixel[4..8].try_into().unwrap()));
            b.push(f32::from_le_bytes(pixel[8..12].try_into().unwrap()));
        }
    }

    Ok((r, g, b))
}

/// Build an `Rgba32F` image from separate `f32` RGB channel slices.
///
/// The slices must each contain exactly `width * height` values. The supplied
/// pixel format, gamut, and transfer are stored as image metadata; samples are
/// copied without color conversion, and alpha is set to 1.0.
pub (crate) fn rgb_to_raw_image(
    width: u32,
    height: u32,
    r: &[f32],
    g: &[f32],
    b: &[f32],
    format: UhdrPixelFormat,
    gamut: UhdrColorGamut,
    transfer: UhdrColorTransfer,
) -> Result<RawImage, String> {
    if format != UhdrPixelFormat::Rgba32F {
        return Err(format!(
            "rgb_to_raw_image requires Rgba32F output, got {format:?}"
        ));
    }

    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "RawImage dimensions overflow".to_string())?;
    if r.len() != pixel_count || g.len() != pixel_count || b.len() != pixel_count {
        return Err(format!(
            "RGB channel lengths must all equal width * height ({pixel_count}); got R={}, G={}, B={}",
            r.len(),
            g.len(),
            b.len()
        ));
    }

    let data_len = pixel_count
        .checked_mul(16)
        .ok_or_else(|| "RawImage byte length overflow".to_string())?;
    let mut data = Vec::with_capacity(data_len);
    for i in 0..pixel_count {
        data.extend_from_slice(&r[i].to_le_bytes());
        data.extend_from_slice(&g[i].to_le_bytes());
        data.extend_from_slice(&b[i].to_le_bytes());
        data.extend_from_slice(&1.0f32.to_le_bytes());
    }

    RawImage::from_data(width, height, format, gamut, transfer, data)
        .map_err(|error| error.to_string())
}
