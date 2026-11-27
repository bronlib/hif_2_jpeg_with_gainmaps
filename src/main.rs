use hif2jpeg::MyApp;

fn main() -> eframe::Result {
    eframe::run_native(
        "hif2jpeg",
        Default::default(),
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}
