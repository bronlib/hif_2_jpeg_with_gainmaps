use crate::decode_hif2rgb444::decode_heif2rgb_o_domain;

use crate::create_png::create_png;
use std::time::Instant;
use zenjpeg::ultrahdr::generate_gainmap_xmp;

use crate::color_spaces::rgb_to_xyz_wide16;
use crate::color_spaces::xyz_to_rgb_wide16;
use crate::simd_multiply::mul_vec_scalar_new_wide16;
use crate::simd_multiply::mul_vec_scalar_wide16;
use crate::yuv_prime::{xyz_to_yuvprime_wide16, yuvprime_to_xyz_wide16};

use crate::create_jpeg_from_2_files::encodemy_zen;

pub(crate) fn create_hdr_from_file(
    file: &str,
    to_file: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let total_start = Instant::now();
    println!("Hello, world!");

    //let (width, height, r,g,b ) = decode_heif2rgb_e_domain("A6701459.HIF")?;
    //let _ = create_png("tmp2.png", width, height, &*r, &*g, &*b);

    let (width, height, r, g, b) = decode_heif2rgb_o_domain(file).unwrap();

    let (x, y, z) = rgb_to_xyz_wide16(&r, &g, &b);

    let (yy, uu, vv) = xyz_to_yuvprime_wide16(&x, &y, &z);

    let ysdr = mul_vec_scalar_new_wide16(&yy, 1.0);
    let yhdr = mul_vec_scalar_new_wide16(&yy, 1.6);

    let (xxsdr, yysdr, zzsdr) = yuvprime_to_xyz_wide16(&ysdr, &uu, &vv);
    let (xxhdr, yyhdr, zzhdr) = yuvprime_to_xyz_wide16(&yhdr, &uu, &vv);

    let (r_sdr, g_sdr, b_sdr) = xyz_to_rgb_wide16(&xxsdr, &yysdr, &zzsdr);
    let (r_hdr, g_hdr, b_hdr) = xyz_to_rgb_wide16(&xxhdr, &yyhdr, &zzhdr);

    encodemy_zen(
        to_file, width, height, &r_hdr, &g_hdr, &b_hdr, &r_sdr, &g_sdr, &b_sdr,
    );

    //let _ = create_png("tmpsdr.png", width, height, &*r_sdr, &*g_sdr, &*b_sdr);

    // let _ = create_png("tmphdr.png", width, height, &*r_hdr, &*g_hdr, &*b_hdr);

    let total = total_start.elapsed();
    println!("Total pipeline time: {:.3?}", total);
    Ok(())
}
