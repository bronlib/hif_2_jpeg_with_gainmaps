use std::path::PathBuf;

use rfd::FileDialog;
use slint::ComponentHandle;

use crate::create_jpeg_from_hdr::create_hdr_from_file;

slint::include_modules!();

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ui = MainWindow::new()?;
    let weak = ui.as_weak();
    ui.on_browse_input({
        let weak = weak.clone();
        move || {
            let selected = FileDialog::new()
                .set_title("Choose HIF/HEIF input image")
                .add_filter("HEIF images", &["hif", "heif", "heic"])
                .pick_file();
            if let (Some(path), Some(ui)) = (selected, weak.upgrade()) {
                ui.set_input_path(path.to_string_lossy().into_owned().into());
            }
        }
    });

    ui.on_browse_output({
        let weak = weak.clone();
        move || {
            let selected = FileDialog::new()
                .set_title("Save Ultra HDR JPEG")
                .set_file_name("gainmapped.jpg")
                .add_filter("JPEG image", &["jpg", "jpeg"])
                .save_file();
            if let (Some(path), Some(ui)) = (selected, weak.upgrade()) {
                ui.set_output_path(path.to_string_lossy().into_owned().into());
            }
        }
    });

    ui.on_encode({
        let weak = weak.clone();
        move || start_encoding(weak.clone())
    });

    ui.run()?;
    Ok(())
}

fn start_encoding(weak: slint::Weak<MainWindow>) {
    let Some(ui) = weak.upgrade() else { return };
    let input = ui.get_input_path().to_string();
    let output = ui.get_output_path().to_string();
    let sdr_gain = match parse_gain(&ui.get_sdr_gain().to_string(), "SDR Gain") {
        Ok(value) => value,
        Err(message) => {
            ui.set_status(message.into());
            return;
        }
    };
    let hdr_gain = match parse_gain(&ui.get_hdr_gain().to_string(), "HDR Gain") {
        Ok(value) => value,
        Err(message) => {
            ui.set_status(message.into());
            return;
        }
    };
    if input.trim().is_empty() || output.trim().is_empty() {
        ui.set_status("Select both an input file and an output file.".into());
        return;
    }
    if same_path(&input, &output) {
        ui.set_status("Input and output must be different files.".into());
        return;
    }

    ui.set_busy(true);
    ui.set_status("Starting encoder…".into());
    drop(ui);

    std::thread::spawn(move || {
        let progress_weak = weak.clone();
        let progress = move |message: &str| {
            let weak = progress_weak.clone();
            let message = message.to_owned();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.set_status(message.into());
                }
            });
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            create_hdr_from_file(&input, &output, sdr_gain, hdr_gain, &progress)
        }));
        let status = match result {
            Ok(Ok(())) => format!("Finished: {output}"),
            Ok(Err(error)) => format!("Encoding failed: {error}"),
            Err(payload) => {
                let detail = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("unexpected panic in the encoder");
                format!("Encoder stopped: {detail}")
            }
        };
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_busy(false);
                ui.set_status(status.into());
            }
        });
    });
}

fn parse_gain(value: &str, label: &str) -> Result<f32, String> {
    let parsed: f32 = value
        .trim()
        .parse()
        .map_err(|_| format!("{label} must be a number."))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("{label} must be greater than zero."));
    }
    Ok(parsed)
}

fn same_path(input: &str, output: &str) -> bool {
    let input = PathBuf::from(input);
    let output = PathBuf::from(output);
    if input == output {
        return true;
    }
    input
        .canonicalize()
        .ok()
        .zip(output.canonicalize().ok())
        .is_some_and(|(a, b)| a == b)
}
