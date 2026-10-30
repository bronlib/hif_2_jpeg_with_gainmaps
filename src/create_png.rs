use image::{Rgb, RgbImage};

pub(crate) fn create_pngu8(
    name: &str,
    width: usize,
    height: usize,
    r: &[u8],
    g: &[u8],
    b: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut img = RgbImage::new(width as u32, height as u32);

    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;

            // Read once, clamp once, convert once
            let rr = r[i];
            let gg = g[i];
            let bb = b[i];

            img.put_pixel(x as u32, y as u32, Rgb([rr, gg, bb]));
        }
    }

    img.save(name)?;
    Ok(())
}

pub(crate) fn create_png(
    name: &str,
    width: usize,
    height: usize,
    r: &[f32],
    g: &[f32],
    b: &[f32],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut img = RgbImage::new(width as u32, height as u32);

    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;

            // Read once, clamp once, convert once
            let rr = (r[i].clamp(0.0, 1.0) * 255.0) as u8;
            let gg = (g[i].clamp(0.0, 1.0) * 255.0) as u8;
            let bb = (b[i].clamp(0.0, 1.0) * 255.0) as u8;

            img.put_pixel(x as u32, y as u32, Rgb([rr, gg, bb]));
        }
    }

    img.save(name)?;
    Ok(())
}
