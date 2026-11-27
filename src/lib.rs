#![doc = include_str!("../README.md")]


mod color_spaces;
mod decode_hif2rgb444;
mod filters;
mod hlg;
mod simd_multiply;
mod yuv_prime;
mod create_png;

pub mod color_transform;
pub mod create_jpeg_from_2_files;
mod create_jpeg_from_hdr;
pub mod enccode_decode;
mod gain_mapping;
mod gainmap_jpeg_fix;
//pub mod rgb2020_709;
pub mod transfer_functions;
pub mod get_headroom;
pub mod pq;
pub mod raw_image_transforms;
pub mod decode_gainmapped_jpg;

use egui::{Color32, RichText};



use eframe::epaint::tessellator::Path;
use eframe::{CreationContext};

use eframe::egui;
use image::imageops::flip_horizontal_in_place;
use image::{DynamicImage, ImageReader, Rgba, ImageBuffer};


use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};


//use eframe::egui::{CentralPanel, ComboBox, Popup, PopupCloseBehavior};
use create_jpeg_from_hdr::{create_hdr_from_file, add_suffix, CreatedJpeg };

use decode_gainmapped_jpg::decode_file;


use robius_file_picker::{FileDialog, LocalFile};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Log to android output
    // while robius-file-picker needs the NativeActivity to launch its fragment.
// The current Slint Android backend stores an Application in ndk-context,
    // while robius-file-picker needs the NativeActivity to launch its fragment.
    unsafe { robius_android_env::set_activity_ptr(app.activity_as_ptr()) };
    let options = eframe::NativeOptions {
        android_app: Some(app),
        viewport: egui::ViewportBuilder::default().with_inner_size([620.0, 880.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc|{
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::<MyApp>::default())
        }),
    );

}



pub struct MyApp {
    picked_path: Option<LocalFile>,
    path_tx: Sender<LocalFile>,
    path_rx: Receiver<LocalFile>,
    spin: bool,
    image_popup_open: bool,
    image_texture: Option<egui::TextureHandle>,
    image_size: Option<egui::Vec2>,
    image_rx: Option<Receiver<CreatedJpeg>>,
    loading: bool,
    gain_mapped_jpeg : Option<Vec<u8>>,
    create_gainmapped_jpg: bool,

}

impl Default for MyApp {
    fn default() -> Self {
        let (path_tx, path_rx) = std::sync::mpsc::channel();

        Self {
            picked_path : None,
            path_tx,
            path_rx,
            spin: false,
            image_popup_open: false,
            image_texture: None,
            image_size: None,
            image_rx: None,
            loading: false,
            gain_mapped_jpeg: None,
            create_gainmapped_jpg: false,
        }
    }
}


impl MyApp {
    pub fn new(cc: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
       let (path_tx, path_rx) = mpsc::channel();
        Self {
            spin: false,
            picked_path :None,
            path_tx,
            path_rx,
            image_popup_open: false,
            image_texture: None,
            image_size : Some(egui::vec2(0.0, 0.0)),
            image_rx : None,
            loading : false,
            gain_mapped_jpeg : None,
            create_gainmapped_jpg : false,
        }
    }
}


impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.with_layout(
                egui::Layout::right_to_left(egui::Align::TOP),
                |ui| {
                    if ui.add(
                        egui::Button::new(
                            egui::RichText::new("X")
                                .color(egui::Color32::WHITE)
                                .strong()
                        )
                            .fill(egui::Color32::RED)
                    ).clicked() {
                        ui.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                },
            );
            ui.horizontal(|ui| {
                ui.label("    ");
                ui.label("Select a HIF, heif or jpg file to decode");
            });
           
            if ui.button("Open file…").clicked()
            {   self.spin = true;
                let tx = self.path_tx.clone();
                FileDialog::new()
                    .add_filter("HIF pictures", &["HIF", "heif", "heic", "jpg"])
                    .pick_file(move |result| {
                        if let Ok(Some(file)) = result {
                            match file.into_local_file() {
                                Ok(local_file) => {
                                    let _ = tx.send(local_file);
                                }
                                Err(err) => eprintln!("Could not access selected file: {err}"),
                            }
                        }
                    })
                    .expect("failed to show file dialog");
            }
            ui.checkbox(
                &mut self.create_gainmapped_jpg,
                "Create gainmapped JPG from the HIF file",
            );
            while let Ok(file) = self.path_rx.try_recv() {
                self.picked_path = Some(file);
                self.spin = false;
            }


            if let Some(selected_file) = self.picked_path.as_ref() {
                let path = selected_file.path();

                if path.is_file() {
                    ui.horizontal(|ui| {
                        ui.label("Picked file:");
                        ui.monospace(path.display().to_string());
                    });
                }

                if path.is_file() && !self.loading && !self.spin {
                    // Keep the LocalFile (and any temporary copy it owns) alive
                    // while the decode thread uses its path.
                    let selected_file = selected_file.clone();
                    let path = selected_file.path().to_path_buf();
                    let (tx, rx) = mpsc::channel();
                    let create_file = self.create_gainmapped_jpg.clone();

                    std::thread::spawn(move || {
                        let _keep_file_alive = selected_file;
                        let choose_file = create_file;
                        let extension = path
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();

                        let created_jpg = if ["jpeg", "jpg"].contains(&extension.as_str()) {

                            let preview = decode_file(&path).expect("Unable to decode jpg file");
                            let jpeg = vec![0u8; 0];
                            CreatedJpeg{
                                preview,
                                jpeg,
                            }
                        } else {
                            let  struc_result= create_hdr_from_file(&path, 1.0, 1.0)
                                .expect("Unable to decode HIF file");

                            if choose_file{
                                    let newname = add_suffix(&path, "_gainmapped", ".jpg");
                                    if let Err(err) = std::fs::write(&newname, &struc_result.jpeg) {
                                        eprintln!("Could not write JPEG: {err}");
                                    } else if let Err(err) = FileDialog::new().save_to_downloads(&newname, |result| {
                                        println!("Saved file: {result:?}");
                                    }) {
                                        eprintln!("Could not start save to Downloads: {err}");
                                    }
                            }else{
                                eprintln!("Not Printing");
                            }
                            struc_result

                        };

                        let _ = tx.send(created_jpg);
                    });

                    self.image_rx = Some(rx);
                    self.loading = true;
                }
            }

            if let Some(rx) = &self.image_rx {
                if let Ok(created_jpg) = rx.try_recv() {
                    let image = created_jpg.preview.to_rgba8();
                    let size = [image.width() as usize, image.height() as usize];
                    println!("width={:?}, height={:?}", image.width(), image.height());
                    let repaint_ctx = ui.clone();

                    let color_image = egui::ColorImage::from_rgba_unmultiplied(
                        size,
                        image.as_raw(),
                    );
                    self.image_texture = Some(ui.load_texture(
                        "selected picture",
                        color_image,
                        egui::TextureOptions::LINEAR,
                    ));
                    self.image_size =
                        Some(egui::vec2(size[0] as f32, size[1] as f32));
                    repaint_ctx.request_repaint();
                    self.image_popup_open=true;
                    self.loading = false;
                }
            }

            if self.image_popup_open {
                self.spin = false;
                let preview_viewport = egui::ViewportId::from_hash_of("selected-picture-preview");
                let preview_builder = egui::ViewportBuilder::default()
                    .with_title("Selected picture")
                    .with_inner_size([640.0, 480.0])
                    .with_min_inner_size([240.0, 180.0]);

                ui.show_viewport_immediate(preview_viewport, preview_builder, |viewport_ctx, _class| {
                    if viewport_ctx.input(|input| input.viewport().close_requested()) {
                        self.image_popup_open = false;
                    }

                    egui::CentralPanel::default().show(viewport_ctx, |ui| {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::TOP),
                            |ui| {
                                if ui.add(
                                    egui::Button::new(
                                        egui::RichText::new("X")
                                            .color(egui::Color32::WHITE)
                                            .strong()
                                    )
                                        .fill(egui::Color32::RED)
                                ).clicked() {
                                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            },
                        );
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
            }else{
                self.picked_path = None;
               
            }


            if self.spin {
                ui.spinner();

            }

            if self.loading{
                ui.spinner();
                ui.label("Decoding picture…");
            }


        });


    }
}
