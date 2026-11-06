#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod color_spaces;
mod decode_hif2rgb444;
mod filters;
mod hlg;
mod pq;
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
pub mod raw_image_transforms;

use create_jpeg_from_hdr::create_hdr_from_file;
use slint::{Image as SlintImage, Rgb8Pixel, SharedPixelBuffer};

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let main_window = MainWindow::new()?;
    let image_window = ImageWindow::new()?;

    let main_window_weak = main_window.as_weak();
    let image_window_weak = image_window.as_weak();

    main_window.on_select_file(move || {
        let Some(main_window) = main_window_weak.upgrade() else {
            return;
        };

        let Some(path) = rfd::FileDialog::new()
            .add_filter("HIF pictures", &["HIF", "heif", "heic"])
            .pick_file()
        else {
            return;
        };

        let create_gainmapped_jpg = main_window.get_create_gainmapped_jpg();
        main_window.set_error_message("".into());
        main_window.set_loading(true);

        let main_window_weak = main_window.as_weak();
        let image_window_weak = image_window_weak.clone();
        std::thread::spawn(move || {
            let result = create_hdr_from_file(&path, create_gainmapped_jpg)
                .map_err(|error| error.to_string());

            if let Err(error) = slint::invoke_from_event_loop(move || {
                let Some(main_window) = main_window_weak.upgrade() else {
                    return;
                };
                main_window.set_loading(false);

                match result {
                    Ok(image) => {
                        let rgb = image.to_rgb8();
                        let pixels = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(
                            rgb.as_raw(),
                            rgb.width(),
                            rgb.height(),
                        );
                        if let Some(image_window) = image_window_weak.upgrade() {
                            image_window.set_picture(SlintImage::from_rgb8(pixels));
                            if let Err(error) = image_window.show() {
                                main_window.set_error_message(
                                    format!("Could not show image window: {error}").into(),
                                );
                            }
                        }
                    }
                    Err(error) => {
                        main_window.set_error_message(
                            format!("Could not process picture: {error}").into(),
                        );
                    }
                }
            }) {
                eprintln!("Could not notify Slint UI: {error}");
            }
        });
    });

    main_window.run()?;
    Ok(())
}
