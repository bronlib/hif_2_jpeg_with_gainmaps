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

use std::env;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    print_usage(&args);
    match args.len() {
        3 => {
            println!("File mode");
            let hdr = Path::new(&args[0]);
            let sdr = Path::new(&args[1]);
            let outp = Path::new(&args[2]);
            if ! hdr.is_file(){
                println!("{:?} is not a file", hdr);
                print_usage(&args);
                return Ok(());
            };
            if ! sdr.is_file(){
                println!("{:?} is not a file", sdr);
                print_usage(&args);
                return Ok(());
            }
            let res = create_from_2_files(hdr, sdr, outp);
            res.expect("create_from_2_files failed");
        }
        4 => {
            // 1 input file + 2 floats + 1 output file
            println!("Float mode");
            let hdr = Path::new(&args[0]);
            if ! hdr.is_file(){
                println!("{:?} is not a file", hdr);
                print_usage(&args);
                return Ok(());
            };
            let hdr_gain: f32 = args[1].parse().expect("Expected float");
            let sdr_gain: f32= args[2].parse().expect("Expected float");
            let outp = Path::new(&args[3]);
            let res = create_hdr_from_file(hdr,hdr_gain, sdr_gain, outp );
            res.expect("create_hdr_from_file failed");
        }
        _ => {
            println!("Invalid number of arguments");
            print_usage(&args);
            return Ok(());
        }
    };

    Ok(())
}

fn print_usage(args: &Vec<String>) {
    println!("Found : {:?}",args );
    println!(
        "Usage:\n  \
         hif2jpeg   <hdr.jpg> <sdr.jpg> <output.jpg>\n  \
         hif2jpeg  <input.hif> <gain_hdr> <gain_sdr> <output.jpg>\n"
    );
}
