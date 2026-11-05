mod color_spaces;
mod decode_hif2rgb444;
mod filters;
mod gui;
mod hlg;
mod simd_multiply;
mod yuv_prime;

pub mod color_transform;
pub mod create_jpeg_from_2_files;
mod create_jpeg_from_hdr;
pub mod enccode_decode;
mod gain_mapping;
mod gainmap_jpeg_fix;
pub mod transfer_functions;
pub mod get_headroom;



fn main() {
    gui::run().expect("Slint application failed");
}
