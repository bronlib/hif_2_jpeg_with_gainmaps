pub(crate) fn upsample_422(chroma: &[f32], stride: usize, width: usize, height: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; width * height];

    for y in 0..height {
        for x in 0..width {
            // 4:2:2 → chroma has half horizontal resolution
            let cx = x / 2;
            let v = chroma[y * stride + cx] as f32;
            out[y * width + x] = v;
        }
    }

    out
}

/// Upsample a 4:2:0 chroma plane to full resolution.
///
/// `chroma`  = &[f32] chroma samples
/// `stride`  = number of chroma samples per row (width/2)
/// `width`   = full-resolution width
/// `height`  = full-resolution height
///
/// Output: Vec<f32> of size width * height
pub(crate) fn upsample_420(chroma: &[f32], stride: usize, width: usize, height: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; width * height];

    for y in 0..height {
        // 4:2:0 → chroma has half vertical resolution
        let cy = y / 2;

        for x in 0..width {
            // 4:2:0 → chroma has half horizontal resolution
            let cx = x / 2;

            let v = chroma[cy * stride + cx] as f32;
            out[y * width + x] = v;
        }
    }

    out
}
