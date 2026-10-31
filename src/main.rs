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

use create_jpeg_from_2_files::create_from_2_files;
use create_jpeg_from_hdr::create_hdr_from_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let stack_jpegs = "stack-jpegs";
    let transcode_hif = "transcode-hif";

    match args.as_slice() {
        [command] if command == "--help" || command == "-h" => {
            print_usage(stack_jpegs, transcode_hif);
        }
        [command, hdr, sdr, output] if command == stack_jpegs => {
            create_from_2_files(hdr, sdr, output)?;
        }
        [command, input, gain_hdr, gain_sdr, output] if command == transcode_hif => {
            let gain_hdr: f32 = gain_hdr.parse()?;
            let gain_sdr: f32 = gain_sdr.parse()?;
            create_hdr_from_file(input, gain_hdr, gain_sdr, output)?;
        }
        [] => print_usage(stack_jpegs, transcode_hif),
        _ => {
            print_usage(stack_jpegs, transcode_hif);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid command or arguments; use --help for usage",
            )
            .into());
        }
    }

    Ok(())
}

fn print_usage(stack: &str, hdr: &str) {
    println!(
        "Usage:\n  hif2jpeg {} <hdr.jpg> <sdr.jpg> <output.jpg>\n  \
         hif2jpeg {} <input.hif> <gain_hdr> <gain_sdr> <output.jpg>\n\nExamples:\n \
         hif2jpeg {} gainmappped_colorful.jpg gainmapped_outline.jpg gainmapped_rust3.jpg\n  \
         hif2jpeg {} A6704110.HIF 1.6 1.0 A6704110_a.jpg", stack, hdr, stack, hdr
    );
}
