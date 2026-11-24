use heic_decoder::{decode, ColorInfo};
use std::fs;
use crate::color_spaces::{bt2020_yuv_to_rgb_wide16, bt709_yuv_to_rgb_wide16};
use crate::color_transform::{bt2020_to_bt709, bt709_to_bt2020 };
use crate::decode_gainmapped_jpg::DecodedJpeg;
use crate::filters::upsample_420;
use crate::filters::upsample_422;
use crate::hlg::hlg_eotf_wide16;
use std::path::PathBuf;
use oxideav_pixfmt::transfer::bt1886_eotf;
use ultrahdr_core::ColorTransfer;
use zenjpeg::ultrahdr::{UhdrColorGamut, UhdrColorTransfer, UhdrPixelFormat};
use oxideav_pixfmt::transfer::{ bt1886_inverse_eotf};

use crate::transfer_functions::apply_transfer;
use crate::pq::pq_eotf_wide16;

use crate::raw_image_transforms::{raw_image_to_rgb, rgb_to_raw_image};
use image::{DynamicImage};

use crate::create_jpeg_from_2_files::rgba32f_to_rgb8;

//use crate::decode_gainmapped_jpg::DecodedJpeg;

pub (crate) fn decode_heif2img( name: &PathBuf)->Result<DynamicImage, String>{

    //this creates a raw image in bt.2020 linear domain
    let (width, height, r, g, b) = decode_heif2rgb_o_domain(name).unwrap();
    println!("width={width}, height={height}");
    let hdr = rgb_to_raw_image(width, height, &r, &g, &b,
                               UhdrPixelFormat::Rgba32F,
                              UhdrColorGamut::Bt2020,
                              UhdrColorTransfer::Linear).unwrap();

    //transfer to bt709
    let hdr709=bt2020_to_bt709(&hdr);
    //apply itu  ->e
    let mut sdr_e = apply_transfer(&hdr709, &bt1886_inverse_eotf);
    sdr_e.transfer = UhdrColorTransfer::Srgb;
    let sdr_8 = rgba32f_to_rgb8(&sdr_e);
    let mut final_jpeg = DecodedJpeg::default();
    final_jpeg.width = width as u32;
    final_jpeg.height = height  as u32;
    final_jpeg.has_gain_map = false;
    final_jpeg.gain_map = None;
    final_jpeg.metadata = None;
    final_jpeg.hdr_linear_bt2020 =  None;



    final_jpeg.rgb = sdr_8.data;

    final_jpeg.create_dyn()

}

pub(crate) fn decode_heif2yuv_e_domain(
    name: &PathBuf,
) -> Result<(Option<ColorInfo>, usize, usize, Vec<f32>, Vec<f32>, Vec<f32>), Box<dyn std::error::Error>> {
    let data = fs::read(&name)?;
    let image = decode(&data)?;

    let color_info = image.metadata.color_info;
    println!("color info: {:?}", color_info);

    let width = image.width as usize;
    let height = image.height as usize;
    let divider = if color_info==None
                        { 255.0f32
                        }else {
                          let info = color_info.as_ref().unwrap();
                          if info.full_range==true{
                              1023.0f32
                          }else{
                              255.0f32
                          }
                        };

    let y_norm = image
        .y
        .iter()
        .map(|v| *v as f32 / divider)
        .collect::<Vec<f32>>();
    let mut cb_norm = image
        .cb
        .iter()
        .map(|v| *v as f32 / divider)
        .collect::<Vec<f32>>();
    let mut cr_norm = image
        .cr
        .iter()
        .map(|v| *v as f32 / divider)
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
        other => {
            panic!("Unsupported chroma format: {}", other);
        }
    };

    Ok((color_info, width, height, y_norm, cb_full, cr_full))
}

pub(crate) fn decode_heif2rgb_e_domain(
    name: &PathBuf,
) -> Result<(Option<ColorInfo>, usize, usize, Vec<f32>, Vec<f32>, Vec<f32>), Box<dyn std::error::Error>> {
    let (color_info, width, height, y, cb, cr) = decode_heif2yuv_e_domain(name)?;


    let (r, g, b) = if color_info==None
                                                    {bt709_yuv_to_rgb_wide16(&y, &cb, &cr)}
                                                  else
                                                    {
                                                        let info = color_info.as_ref().unwrap();
                                                        if info.color_primaries==1{
                                                            bt709_yuv_to_rgb_wide16(&y, &cb, &cr)
                                                        }else{
                                                           bt2020_yuv_to_rgb_wide16(&y, &cb, &cr)
                                                        }
                                                    };

    Ok((color_info, width, height, r, g, b))
}

pub(crate) fn decode_heif2rgb_o_domain(
    name: &PathBuf,
) -> Result<(usize, usize, Vec<f32>, Vec<f32>, Vec<f32>), Box<dyn std::error::Error>> {
    let (color_info, width, height, r, g, b) = decode_heif2rgb_e_domain(name)?;

    if color_info.is_none() {
        let sdr_709 = rgb_to_raw_image(
            width,
            height,
            &r,
            &g,
            &b,
            UhdrPixelFormat::Rgba32F,
            UhdrColorGamut::Bt709,
            UhdrColorTransfer::Srgb,
        )?;

        let mut sdr_e = apply_transfer(&sdr_709, &bt1886_eotf);
        sdr_e.transfer = ColorTransfer::Linear;
        let sdr2020 = bt709_to_bt2020(&sdr_e);
        let (r_ll, g_ll, b_ll) = raw_image_to_rgb(&sdr2020)?;

        Ok((width, height, r_ll, g_ll, b_ll))
    } else {
        let info = color_info.as_ref().unwrap();

        let (r_ll, g_ll, b_ll) = match info.transfer_characteristics {
            16 => (
                pq_eotf_wide16(&r),
                pq_eotf_wide16(&g),
                pq_eotf_wide16(&b),
            ),
            18 => (
                hlg_eotf_wide16(&r),
                hlg_eotf_wide16(&g),
                hlg_eotf_wide16(&b),
            ),
            1 | 6 | 14 | 15 =>{
            let sdr_709 = rgb_to_raw_image(
                width,
                height,
                &r,
                &g,
                &b,
                UhdrPixelFormat::Rgba32F,
                UhdrColorGamut::Bt709,
                UhdrColorTransfer::Srgb,
            )?;
            println!("Going through default hief from 709 to linear");
            let mut sdr_e = apply_transfer(&sdr_709, &bt1886_eotf);
            sdr_e.transfer = ColorTransfer::Linear;
            let sdr2020 = bt709_to_bt2020(&sdr_e);
              raw_image_to_rgb(&sdr2020).unwrap()
            } ,
            code => return Err(format!("Unsupported transfer characteristic: {code}").into()),
        };

        Ok((width, height, r_ll, g_ll, b_ll))
    }
}
