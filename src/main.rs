mod app;
mod config;
mod sim;
mod terrain;
mod ui;

fn main() -> eframe::Result<()> {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MaRTIni v3 — Mobility and Research Testbed Initiative")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "MaRTIni v3",
        options,
        Box::new(|_cc| Ok(Box::new(app::MaRTIniApp::default()))),
    )
}
