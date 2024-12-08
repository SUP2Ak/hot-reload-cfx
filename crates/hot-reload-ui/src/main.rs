mod app;
mod utils;

use app::HotReloadApp;
use eframe::egui;
use tokio::runtime::Runtime;
use std::sync::Arc;

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt::init();
    let runtime = Arc::new(Runtime::new().expect("Failed to create Tokio runtime"));

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

    let runtime_for_block = runtime.clone();
    let runtime_for_app = runtime.clone();

    eframe::run_native(
        "FiveM Hot Reload",
        options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(runtime_for_block.block_on(async {
                HotReloadApp::new(cc, runtime_for_app).await
            }))
        }),
    )
}
