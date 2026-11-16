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
//pub mod rgb2020_709;
pub mod transfer_functions;
pub mod get_headroom;
pub mod raw_image_transforms;

use eframe::egui;
use std::path::PathBuf;
use create_jpeg_from_hdr::create_hdr_from_file;

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Image viewer",
        options,
        Box::new(|_creation_context| Ok(Box::<ImageViewerApp>::default())),
    )
}

#[derive(Default)]
struct ImageViewerApp {
    image_popup_open: bool,
    image_texture: Option<egui::TextureHandle>,
    image_size: Option<egui::Vec2>,
    error_message: Option<String>,
    loading: bool,
    image_rx: Option<Receiver<Result<image::DynamicImage, String>>>,
    create_gainmapped_jpg: bool,
}

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Receive completed work on the UI thread, where egui textures can be created.
        if let Some(image_rx) = &self.image_rx {
            match image_rx.try_recv() {
                Ok(result) => {
                    self.image_rx = None;
                    self.loading = false;
                    match result {
                        Ok(image) => {
                            let image = image.to_rgba8();
                            let size = [image.width() as usize, image.height() as usize];
                            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                size,
                                image.as_raw(),
                            );
                            self.image_texture = Some(ctx.load_texture(
                                "selected picture",
                                color_image,
                                egui::TextureOptions::LINEAR,
                            ));
                            self.image_size =
                                Some(egui::vec2(size[0] as f32, size[1] as f32));
                            self.image_popup_open = true;
                            self.error_message = None;
                        }
                        Err(error) => {
                            self.error_message = Some(format!("Could not open picture: {error}"));
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.image_rx = None;
                    self.loading = false;
                    self.error_message = Some("Image processing stopped unexpectedly.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    if ui.button("Select file").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Pictures", &["HIF","heif", "heic"])
                            .pick_file()
                        {
                            self.show_selected_file(ctx, path);
                        }
                    }
                    ui.checkbox(
                        &mut self.create_gainmapped_jpg,
                        "Create gainmapped JPG from the HIF file",
                    );
                });
                if self.loading {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Processing picture…");
                    });
                }
                if let Some(message) = &self.error_message {
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::LIGHT_RED, message);
                }
            });
        });

        if self.image_popup_open {
            let preview_viewport = egui::ViewportId::from_hash_of("selected-picture-preview");
            let preview_builder = egui::ViewportBuilder::default()
                .with_title("Selected picture")
                .with_inner_size([640.0, 480.0])
                .with_min_inner_size([240.0, 180.0]);

            ctx.show_viewport_immediate(preview_viewport, preview_builder, |viewport_ctx, _class| {
                if viewport_ctx.input(|input| input.viewport().close_requested()) {
                    self.image_popup_open = false;
                }

                egui::CentralPanel::default().show(viewport_ctx, |ui| {
                    if let (Some(texture), Some(size)) = (&self.image_texture, self.image_size) {
                        // Fit the picture inside the popup while preserving its aspect ratio.
                        let available = ui.available_size();
                        let scale = (available.x / size.x).min(available.y / size.y).min(1.0);
                        ui.centered_and_justified(|ui| {
                            ui.image((texture.id(), size * scale));
                        });
                    }
                });
            });
        }

        if self.loading {
            // Keep the spinner animated while the worker is processing the image.
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

impl ImageViewerApp {
    /// Run the potentially slow image conversion away from the UI thread.
    fn show_selected_file(&mut self, ctx: &egui::Context, path: PathBuf) {
        let (tx, rx) = mpsc::channel();
        let repaint_ctx = ctx.clone();
        let create_gainmapped_jpg = self.create_gainmapped_jpg;
        std::thread::spawn(move || {
            let result = create_hdr_from_file(&path, create_gainmapped_jpg)
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
            repaint_ctx.request_repaint();
        });

        self.image_rx = Some(rx);
        self.loading = true;
        self.error_message = None;
    }
}
