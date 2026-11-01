mod color_spaces;
mod create_png;
mod decode_hif2rgb444;
mod filters;
mod hlg;
mod simd_multiply;
mod yuv_prime;

pub mod color_transform;
pub mod create_jpeg_from_2_files;
mod create_jpeg_from_hdr;
pub mod enccode_decode;
mod gain_mapping;
mod gainmap_jpeg_fix;
//pub mod rgb2020_709;
pub mod transfer_functions;
pub mod get_headroom;

use create_jpeg_from_2_files::create_from_2_files;
use create_jpeg_from_hdr::create_hdr_from_file;

fn main() {
    //create_from_2_files("wel.jpg", "niet.jpg", "totwell.jpg");
    create_from_2_files(
        "gainmappped_colorful.jpg",
        "gainmapped_outline.jpg",
        "gainmapped_rust3.jpg",
    )
    .unwrap();

    create_hdr_from_file("A6704110.HIF", 1.6, 1.0, "A6704110_a.jpg").unwrap();
}
