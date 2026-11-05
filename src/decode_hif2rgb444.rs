use heic_decoder::decode;
use std::fs;

use crate::color_spaces::bt2020_yuv_to_rgb_wide16;
use crate::filters::upsample_420;
use crate::filters::upsample_422;
use crate::hlg::hlg_eotf_wide16;

pub(crate) fn decode_heif2yuv_e_domain(
    name: &str,
) -> Result<(usize, usize, Vec<f32>, Vec<f32>, Vec<f32>), Box<dyn std::error::Error>> {
    let data = fs::read(&name)?;
    let image = decode(&data)?;

    let width = image.width as usize;
    let height = image.height as usize;

    let y_norm = image
        .y
        .iter()
        .map(|v| *v as f32 / 1023.0)
        .collect::<Vec<f32>>();
    let mut cb_norm = image
        .cb
        .iter()
        .map(|v| *v as f32 / 1023.0)
        .collect::<Vec<f32>>();
    let mut cr_norm = image
        .cr
        .iter()
        .map(|v| *v as f32 / 1023.0)
        .collect::<Vec<f32>>();

    cb_norm.iter_mut().for_each(|v| *v -= 0.5);
    cr_norm.iter_mut().for_each(|v| *v -= 0.5);

    let (cb_full, cr_full) = match image.chroma_format_idc {
        1 => (
            upsample_420(&cb_norm, width / 2, width, height),
            upsample_420(&cr_norm, width / 2, width, height),
        ),
        2 => (
            upsample_422(&cb_norm, width / 2, width, height),
            upsample_422(&cr_norm, width / 2, width, height),
        ),
        3 => (cb_norm.clone(), cr_norm.clone()),
        other => return Err(format!("Unsupported HEIF chroma format: {other}").into()),
    };

    Ok((width, height, y_norm, cb_full, cr_full))
}

pub(crate) fn decode_heif2rgb_e_domain(
    name: &str,
) -> Result<(usize, usize, Vec<f32>, Vec<f32>, Vec<f32>), Box<dyn std::error::Error>> {
    let (width, height, y, cb, cr) = decode_heif2yuv_e_domain(name)?;

    let (r, g, b) = bt2020_yuv_to_rgb_wide16(&y, &cb, &cr);

    Ok((width, height, r, g, b))
}

pub(crate) fn decode_heif2rgb_o_domain(
    name: &str,
) -> Result<(usize, usize, Vec<f32>, Vec<f32>, Vec<f32>), Box<dyn std::error::Error>> {
    let (width, height, r, g, b) = decode_heif2rgb_e_domain(name)?;

    /*
    let r_ll:Vec<f32>  = r.iter().map(|&v| hlg_eotf(v)).collect::<Vec<f32>>();
    let g_ll:Vec<f32>  = g.iter().map(|&v| hlg_eotf(v)).collect::<Vec<f32>>();
    let b_ll:Vec<f32>  = b.iter().map(|&v| hlg_eotf(v)).collect::<Vec<f32>>();
    */

    let r_ll: Vec<f32> = hlg_eotf_wide16(&r);
    let g_ll: Vec<f32> = hlg_eotf_wide16(&g);
    let b_ll: Vec<f32> = hlg_eotf_wide16(&b);

    Ok((width, height, r_ll, g_ll, b_ll))
}
