use crate::decode_hif2rgb444::decode_heif2rgb_o_domain;

use std::time::Instant;
use std::path::PathBuf;

use crate::color_spaces::rgb_to_xyz_wide16;
use crate::color_spaces::xyz_to_rgb_wide16;
use crate::simd_multiply::mul_vec_scalar_new_wide16;
use crate::yuv_prime::{xyz_to_yuvprime_wide16, yuvprime_to_xyz_wide16};

use crate::create_jpeg_from_2_files::encodemy_zen;
use crate::hlg::{hlg_oetf_wide16, hlg_eotf_wide16};
use image::{DynamicImage, ImageResult};

pub fn add_suffix(path: &PathBuf, suffix: &str, new_ext:&str) -> PathBuf {
    let stem = path.file_stem()
        .unwrap_or_default()
        .to_string_lossy();

    let ext = path.extension()
        .unwrap_or_default()
        .to_string_lossy();

    if new_ext != ext && new_ext.len()>0 {
        path.with_file_name(format!("{stem}{suffix}.{new_ext}"))
    }else {
        path.with_file_name(format!("{stem}{suffix}.{ext}"))
    }
}


pub struct CreatedJpeg{
    pub preview : DynamicImage,
    pub jpeg : Vec<u8>,
}

pub(crate) fn create_hdr_from_file(
    file: &PathBuf,
    gain_hdr : f32,
    gain_sdr : f32,
) -> Result<CreatedJpeg, String>{
    let total_start = Instant::now();
    println!("Hello, world!");

    let (width, height, r, g, b) = decode_heif2rgb_o_domain(file).unwrap();

    let (x, y, z) = rgb_to_xyz_wide16(&r, &g, &b);

    let (yy, uu, vv) = xyz_to_yuvprime_wide16(&x, &y, &z);
    
    let yy_oe = hlg_oetf_wide16(&yy);

    let max_yy_oe = yy_oe.iter().copied().reduce(f32::max).unwrap();
    if gain_hdr>1.0/max_yy_oe {
        println!("Warning, max y is {} so  gain_hdr*max y={}", max_yy_oe, gain_hdr*max_yy_oe);
    }

    //Perform the multiplication in perceptually uniform space
    let ysdr_e = mul_vec_scalar_new_wide16(&yy_oe, gain_sdr);
    let yhdr_e = mul_vec_scalar_new_wide16(&yy_oe, gain_hdr);
    
    let ysdr = hlg_eotf_wide16(&ysdr_e);
    let yhdr =hlg_eotf_wide16(&yhdr_e);
    

    let (xxsdr, yysdr, zzsdr) = yuvprime_to_xyz_wide16(&ysdr, &uu, &vv);
    let (xxhdr, yyhdr, zzhdr) = yuvprime_to_xyz_wide16(&yhdr, &uu, &vv);

    let (r_sdr, g_sdr, b_sdr) = xyz_to_rgb_wide16(&xxsdr, &yysdr, &zzsdr);
    let (r_hdr, g_hdr, b_hdr) = xyz_to_rgb_wide16(&xxhdr, &yyhdr, &zzhdr);

    let total = total_start.elapsed();
    println!("Total pipeline time: {:.3?}", total);

    encodemy_zen(
        width, height, &r_hdr, &g_hdr, &b_hdr, &r_sdr, &g_sdr, &b_sdr,
    )


}
