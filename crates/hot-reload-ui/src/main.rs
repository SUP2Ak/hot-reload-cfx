mod app;

use app::HotReloadApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();

    let icon = image::load_from_memory(include_bytes!("../../../assets/supv.ico"))
        .expect("Erreur lors du chargement de l'icône!")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("FiveM Hot Reload")
            .with_icon(egui::IconData {
                rgba: icon.into_raw(),
                width,
                height,
            }),
        ..Default::default()
    };

    eframe::run_native(
        "FiveM Hot Reload",
        options,
        Box::new(|cc| Box::new(HotReloadApp::new(cc))),
    )
}